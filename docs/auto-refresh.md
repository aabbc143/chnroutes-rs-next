# Automatic data refresh

The first stage of automatic refresh is to give both supported data sources the same seven-day cache policy.

- APNIC: cache valid for seven days; expired cache triggers a fresh download.
- chnroutes2: cache valid for seven days; expired cache triggers a fresh download.

The cache refreshes when the CLI needs the source data. Background scheduling and automatic route reconciliation are intentionally separate follow-up features.
