use std::collections::HashSet;

use chnroutes::source::{apnic, chnroutes2};

#[test]
#[ignore = "requires network access"]
fn compare_sources() {
    let apnic_data = apnic::fetch_ip_data().expect("failed to fetch APNIC data");
    let chnroutes2_data =
        chnroutes2::fetch_ip_data().expect("failed to fetch chnroutes2 data");

    let apnic_set: HashSet<_> = apnic_data.iter().copied().collect();
    let chnroutes2_set: HashSet<_> = chnroutes2_data.iter().copied().collect();

    let common = apnic_set.intersection(&chnroutes2_set).count();
    let apnic_only = apnic_set.difference(&chnroutes2_set).count();
    let chnroutes2_only = chnroutes2_set.difference(&apnic_set).count();

    println!("=== Source comparison ===");
    println!("APNIC CIDRs:       {}", apnic_set.len());
    println!("chnroutes2 CIDRs:  {}", chnroutes2_set.len());
    println!("Common CIDRs:      {}", common);
    println!("APNIC only:        {}", apnic_only);
    println!("chnroutes2 only:   {}", chnroutes2_only);
}
