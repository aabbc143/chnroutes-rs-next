#[cfg(windows)]
pub mod win {
    use std::ffi::{OsStr, OsString};
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::net::Ipv4Addr;
    use std::path::PathBuf;
    use std::str::FromStr;
    use std::time::Duration;
    use tokio::sync::mpsc;
    use windows_service::{
        define_windows_service,
        service::{
            ServiceAccess, ServiceControl, ServiceControlAccept, ServiceErrorControl,
            ServiceExitCode, ServiceInfo, ServiceStartType, ServiceState, ServiceStatus,
            ServiceType,
        },
        service_control_handler::{self, ServiceControlHandlerResult},
        service_dispatcher,
        service_manager::{ServiceManager, ServiceManagerAccess},
    };

    const SERVICE_NAME: &str = "chnroutes-rs-next";
    const SERVICE_DISPLAY_NAME: &str = "chnroutes-rs-next Route Persistence Service";
    const CHECK_INTERVAL_HOURS: u64 = 12;
    const TARGET_PROBE_IP: Ipv4Addr = Ipv4Addr::new(223, 5, 5, 5);

    define_windows_service!(ffi_service_main, my_service_main);

    /// RAII 状态管理器，支持 SCM Checkpoint 心跳机制
    struct ServiceStatusGuard<F>
    where
        F: FnMut(ServiceStatus) -> Result<(), windows_service::Error>,
    {
        set_status_fn: F,
        stopped_reported: bool,
        checkpoint: u32,
    }

    impl<F> ServiceStatusGuard<F>
    where
        F: FnMut(ServiceStatus) -> Result<(), windows_service::Error>,
    {
        fn new(set_status_fn: F) -> Self {
            Self {
                set_status_fn,
                stopped_reported: false,
                checkpoint: 0,
            }
        }

        fn report(&mut self, state: ServiceState, exit_code: ServiceExitCode, wait_hint: Duration) {
            let controls_accepted = if state == ServiceState::Running {
                ServiceControlAccept::STOP
            } else {
                ServiceControlAccept::empty()
            };

            if state == ServiceState::StartPending || state == ServiceState::StopPending {
                self.checkpoint += 1;
            } else {
                self.checkpoint = 0;
            }

            let status = ServiceStatus {
                service_type: ServiceType::OWN_PROCESS,
                current_state: state,
                controls_accepted,
                exit_code,
                checkpoint: self.checkpoint,
                wait_hint,
                process_id: None,
            };

            let _ = (self.set_status_fn)(status);
            if state == ServiceState::Stopped {
                self.stopped_reported = true;
            }
        }
    }

    impl<F> Drop for ServiceStatusGuard<F>
    where
        F: FnMut(ServiceStatus) -> Result<(), windows_service::Error>,
    {
        fn drop(&mut self) {
            if !self.stopped_reported {
                self.report(
                    ServiceState::Stopped,
                    ServiceExitCode::Win32(1),
                    Duration::default(),
                );
            }
        }
    }

    fn service_log(message: &str) {
        let log_dir = std::env::var_os("ProgramData")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                std::env::current_exe()
                    .unwrap_or_default()
                    .parent()
                    .unwrap_or_else(|| std::path::Path::new("."))
                    .to_path_buf()
            })
            .join("chnroutes-rs-next");

        let _ = std::fs::create_dir_all(&log_dir);
        let log_path = log_dir.join("chnroutes-service.log");

        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) {
            let _ = writeln!(file, "{}", message);
        }
    }

    pub fn install() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let exe_path = std::env::current_exe()?;
        let service_manager =
            ServiceManager::local_computer(None::<&OsStr>, ServiceManagerAccess::CREATE_SERVICE)?;

        let service_info = ServiceInfo {
            name: OsStr::new(SERVICE_NAME).into(),
            display_name: OsStr::new(SERVICE_DISPLAY_NAME).into(),
            service_type: ServiceType::OWN_PROCESS,
            start_type: ServiceStartType::AutoStart,
            error_control: ServiceErrorControl::Normal,
            executable_path: exe_path,
            launch_arguments: vec![OsStr::new("service").into()],
            dependencies: vec![],
            account_name: None,
            account_password: None,
        };

        let _service = service_manager.create_service(&service_info, ServiceAccess::ALL_ACCESS)?;
        log::info!("Service '{}' installed successfully.", SERVICE_NAME);
        Ok(())
    }

    pub fn remove() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let service_manager =
            ServiceManager::local_computer(None::<&OsStr>, ServiceManagerAccess::CONNECT)?;

        let service = service_manager
            .open_service(SERVICE_NAME, ServiceAccess::DELETE | ServiceAccess::STOP)?;

        let _ = service.stop();
        service.delete()?;

        log::info!("Service '{}' removed successfully.", SERVICE_NAME);
        Ok(())
    }

    pub fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        service_dispatcher::start(SERVICE_NAME, ffi_service_main)?;
        Ok(())
    }

    fn my_service_main(_arguments: Vec<OsString>) {
        if let Err(e) = run_service() {
            log::error!("Service execution error: {}", e);
            service_log(&format!("Service execution error: {e}"));
        }
    }

    fn run_service() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        service_log("=== Service process started ===");

        // 统一采用 Tokio 异步 mpsc Channel 进行 SCM 控制信号跨线程传递
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

        let event_handler = move |control_event| -> ServiceControlHandlerResult {
            match control_event {
                ServiceControl::Stop => {
                    let _ = shutdown_tx.try_send(());
                    ServiceControlHandlerResult::NoError
                }
                _ => ServiceControlHandlerResult::NotImplemented,
            }
        };

        let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)?;
        let mut status_guard =
            ServiceStatusGuard::new(move |status| status_handle.set_service_status(status));

        // 1. 上报初始 StartPending (Checkpoint 1, WaitHint 10s)
        status_guard.report(
            ServiceState::StartPending,
            ServiceExitCode::Win32(0),
            Duration::from_secs(10),
        );

        // 2. 初始化 Tokio Async Runtime
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;

        let run_result = rt.block_on(async {
            service_log("Waiting for network interface and gateway readiness...");

            // 3. 异步心跳探测循环：等待网络就绪，同时持续给 SCM 喂心跳包
            let start = tokio::time::Instant::now();
            let timeout = Duration::from_secs(60);
            let poll_interval = Duration::from_millis(500);
            let mut network_info = None;

            let mut last_scm_update = tokio::time::Instant::now();

            while start.elapsed() < timeout {
                // 每隔 1 秒向 SCM 刷新一次 Checkpoint 和 WaitHint，告知进程活得很好
                if last_scm_update.elapsed() >= Duration::from_secs(1) {
                    status_guard.report(
                        ServiceState::StartPending,
                        ServiceExitCode::Win32(0),
                        Duration::from_secs(10),
                    );
                    last_scm_update = tokio::time::Instant::now();
                }

                if let Ok(info) = crate::route_op::get_best_route_info(TARGET_PROBE_IP) {
                    if info.interface_index > 0 {
                        network_info = Some(info);
                        break;
                    }
                }

                tokio::time::sleep(poll_interval).await;
            }

            // 4. 根据网络探测结果执行自动恢复 (Auto Restore)
            match network_info {
                Some(info) => {
                    let msg = format!(
                        "Network ready (ifindex: {}, gateway: {}). Starting route auto-restore...",
                        info.interface_index, info.next_hop
                    );
                    log::info!("{msg}");
                    service_log(&msg);

                    if let Err(e) = crate::auto_restore().await {
                        let msg = format!("Auto-restore error: {e}");
                        log::error!("{msg}");
                        service_log(&msg);
                    } else {
                        service_log("Auto-restore completed successfully.");
                    }
                }
                None => {
                    let msg = "Network probe timed out after 60s. Deferring route restore to scheduled loop.";
                    log::warn!("{msg}");
                    service_log(msg);
                }
            }

            // 5. 路由恢复彻底完成后，才正式向 SCM 上报 Running 状态！
            status_guard.report(ServiceState::Running, ServiceExitCode::Win32(0), Duration::default());
            service_log("=== Service state transition: RUNNING ===");

            let source_name = match crate::state::State::load() {
                Some(state) => state.source,
                None => crate::source::Source::default().as_str().to_string(),
            };

            let source = match crate::source::Source::from_str(&source_name) {
                Ok(s) => s,
                Err(e) => {
                    let msg = format!("Invalid saved source '{source_name}': {e}");
                    log::error!("{msg}");
                    service_log(&msg);
                    return Err(Box::<dyn std::error::Error + Send + Sync>::from(msg));
                }
            };

            let init_msg = format!("Scheduler loop initialized with source: {}", source.as_str());
            log::info!("{init_msg}");
            service_log(&init_msg);

            let mut interval = tokio::time::interval(Duration::from_secs(CHECK_INTERVAL_HOURS * 3600));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            interval.tick().await;

            // 6. 主事件循环：监听 Stop 信号与 12 小时定时 Refresh
            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        let msg = "SCM stop signal received. Transitioning to StopPending...";
                        log::info!("{msg}");
                        service_log(msg);

                        status_guard.report(
                            ServiceState::StopPending,
                            ServiceExitCode::Win32(0),
                            Duration::from_secs(5),
                        );
                        break;
                    }

                    _ = interval.tick() => {
                        let tick_msg = "Scheduled 12-hour route refresh triggered.";
                        log::info!("{tick_msg}");
                        service_log(tick_msg);

                        match crate::update::update_diff(&source).await {
                            Ok(Some(diff)) => {
                                let msg = format!(
                                    "Route refresh result: +{} added, -{} removed, {} unchanged.",
                                    diff.added.len(),
                                    diff.removed.len(),
                                    diff.unchanged
                                );
                                log::info!("{msg}");
                                service_log(&msg);
                            }
                            Ok(None) => {
                                let msg = "Route refresh skipped (cache fresh).";
                                log::info!("{msg}");
                                service_log(msg);
                            }
                            Err(e) => {
                                let msg = format!("Route refresh failed: {e}");
                                log::error!("{msg}");
                                service_log(&msg);
                            }
                        }
                    }
                }
            }
            Ok(())
        });

        // 7. 优雅退出流程
        match run_result {
            Ok(_) => {
                status_guard.report(
                    ServiceState::Stopped,
                    ServiceExitCode::Win32(0),
                    Duration::default(),
                );
                service_log("=== Service stopped gracefully ===");
                Ok(())
            }
            Err(e) => {
                status_guard.report(
                    ServiceState::Stopped,
                    ServiceExitCode::Win32(1),
                    Duration::default(),
                );
                service_log(&format!("=== Service terminated with error: {e} ==="));
                Err(e)
            }
        }
    }
}
