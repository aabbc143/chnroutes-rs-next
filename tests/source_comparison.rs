use std::collections::HashSet;
use std::net::Ipv4Addr;

use chnroutes::source::{apnic, chnroutes2};
use ipnet::IpNet;

type Interval = (u32, u32);

#[derive(Default)]
struct Slash8Stats {
    networks: usize,
    addresses: u64,
}

#[test]
#[ignore = "requires network access"]
fn compare_sources() {
    let apnic_data = apnic::fetch_ip_data().expect("failed to fetch APNIC data");
    let chnroutes2_data = chnroutes2::fetch_ip_data().expect("failed to fetch chnroutes2 data");

    let apnic_set: HashSet<_> = apnic_data
        .into_iter()
        .filter(|network| matches!(network, IpNet::V4(_)))
        .collect();

    let chnroutes2_set: HashSet<_> = chnroutes2_data.into_iter().collect();

    let common = apnic_set.intersection(&chnroutes2_set).count();
    let apnic_only = apnic_set.difference(&chnroutes2_set).count();
    let chnroutes2_only = chnroutes2_set.difference(&apnic_set).count();

    let apnic_intervals = normalize(&apnic_set);
    let chnroutes2_intervals = normalize(&chnroutes2_set);

    let apnic_addresses = total_addresses(&apnic_intervals);
    let chnroutes2_addresses = total_addresses(&chnroutes2_intervals);

    let intersection_addresses = intersection_size(&apnic_intervals, &chnroutes2_intervals);

    let apnic_only_addresses = apnic_addresses - intersection_addresses;
    let chnroutes2_only_addresses = chnroutes2_addresses - intersection_addresses;

    println!("=== Source comparison ===");
    println!();
    println!("CIDR comparison:");
    println!("  APNIC CIDRs:       {}", apnic_set.len());
    println!("  chnroutes2 CIDRs:  {}", chnroutes2_set.len());
    println!("  Common CIDRs:      {}", common);
    println!("  APNIC only:        {}", apnic_only);
    println!("  chnroutes2 only:   {}", chnroutes2_only);
    println!();
    println!("Normalized address space:");
    println!("  APNIC normalized ranges:      {}", apnic_intervals.len());
    println!(
        "  chnroutes2 normalized ranges: {}",
        chnroutes2_intervals.len()
    );
    println!();
    println!("IPv4 address coverage:");
    println!("  APNIC addresses:              {}", apnic_addresses);
    println!("  chnroutes2 addresses:         {}", chnroutes2_addresses);
    println!("  Intersection:                 {}", intersection_addresses);
    println!("  APNIC only addresses:         {}", apnic_only_addresses);
    println!(
        "  chnroutes2 only addresses:    {}",
        chnroutes2_only_addresses
    );
    println!();
    println!(
        "  APNIC coverage shared:        {:.4}%",
        percentage(intersection_addresses, apnic_addresses)
    );
    println!(
        "  chnroutes2 coverage shared:   {:.4}%",
        percentage(intersection_addresses, chnroutes2_addresses)
    );

    let uncovered = subtract_intervals(&chnroutes2_intervals, &apnic_intervals);

    println!();
    println!("=== chnroutes2 address space outside APNIC ===");
    println!("  Uncovered ranges:             {}", uncovered.len());
    println!(
        "  Uncovered addresses:          {}",
        total_addresses(&uncovered)
    );

    print_slash8_stats(&uncovered);
    print_top_chnroutes2_only(&chnroutes2_set, &apnic_intervals, 20);
}

fn normalize(networks: &HashSet<IpNet>) -> Vec<Interval> {
    let mut intervals: Vec<Interval> = networks
        .iter()
        .filter_map(|network| match network {
            IpNet::V4(network) => {
                let start = ipv4_to_u32(network.network());
                let end = ipv4_to_u32(network.broadcast());

                Some((start, end))
            }
            IpNet::V6(_) => None,
        })
        .collect();

    intervals.sort_unstable_by_key(|interval| interval.0);

    let mut merged: Vec<Interval> = Vec::with_capacity(intervals.len());

    for (start, end) in intervals {
        if let Some((_, current_end)) = merged.last_mut() {
            if start <= current_end.saturating_add(1) {
                if end > *current_end {
                    *current_end = end;
                }

                continue;
            }
        }

        merged.push((start, end));
    }

    merged
}

fn subtract_intervals(source: &[Interval], mask: &[Interval]) -> Vec<Interval> {
    let mut result = Vec::new();
    let mut mask_index = 0;

    for &(source_start, source_end) in source {
        let mut current = source_start;

        while mask_index < mask.len() && mask[mask_index].1 < current {
            mask_index += 1;
        }

        let mut index = mask_index;

        while index < mask.len() {
            let (mask_start, mask_end) = mask[index];

            if mask_start > source_end {
                break;
            }

            if mask_end < current {
                index += 1;
                continue;
            }

            if mask_start > current {
                let uncovered_end = mask_start - 1;

                if uncovered_end >= current {
                    result.push((current, uncovered_end));
                }
            }

            if mask_end >= source_end {
                current = source_end.saturating_add(1);
                break;
            }

            current = mask_end + 1;
            index += 1;
        }

        if current <= source_end {
            result.push((current, source_end));
        }

        mask_index = index.min(mask.len());
    }

    result
}

fn print_slash8_stats(intervals: &[Interval]) {
    let mut stats: Vec<Slash8Stats> = (0..=255).map(|_| Slash8Stats::default()).collect();

    for &(start, end) in intervals {
        let mut current = start;

        loop {
            let first_octet = (current >> 24) as usize;

            let octet_end = if first_octet == 255 {
                u32::MAX
            } else {
                (((first_octet + 1) as u32) << 24) - 1
            };

            let part_end = end.min(octet_end);

            stats[first_octet].networks += 1;
            stats[first_octet].addresses += u64::from(part_end) - u64::from(current) + 1;

            if part_end >= end {
                break;
            }

            current = part_end + 1;
        }
    }

    println!();
    println!("=== chnroutes2 extra address space by /8 ===");
    println!("{:<6} {:>12} {:>18}", "/8", "Ranges", "Addresses");

    for (octet, stat) in stats.iter().enumerate() {
        if stat.addresses == 0 {
            continue;
        }

        println!(
            "{:<6} {:>12} {:>18}",
            format!("{octet}.0.0.0/8"),
            stat.networks,
            stat.addresses
        );
    }
}

fn print_top_chnroutes2_only(
    chnroutes2_set: &HashSet<IpNet>,
    apnic_intervals: &[Interval],
    limit: usize,
) {
    let mut ranges = chnroutes2_set
        .iter()
        .filter_map(|network| match network {
            IpNet::V4(network) => {
                let start = ipv4_to_u32(network.network());
                let end = ipv4_to_u32(network.broadcast());

                let uncovered = subtract_single_interval((start, end), apnic_intervals);
                let uncovered_addresses = total_addresses(&uncovered);

                if uncovered_addresses == 0 {
                    None
                } else {
                    Some((network, uncovered_addresses))
                }
            }
            IpNet::V6(_) => None,
        })
        .collect::<Vec<_>>();

    ranges.sort_unstable_by_key(|a| std::cmp::Reverse(a.1));

    println!();
    println!("=== Top chnroutes2 prefixes by actual uncovered addresses ===");
    println!("{:<20} {:>18}", "CIDR", "Uncovered");

    for (network, uncovered_addresses) in ranges.into_iter().take(limit) {
        println!("{:<20} {:>18}", network, uncovered_addresses);
    }
}

fn subtract_single_interval(target: Interval, mask: &[Interval]) -> Vec<Interval> {
    subtract_intervals(&[target], mask)
}

fn intersection_size(left: &[Interval], right: &[Interval]) -> u64 {
    let mut i = 0;
    let mut j = 0;
    let mut total = 0u64;

    while i < left.len() && j < right.len() {
        let start = left[i].0.max(right[j].0);
        let end = left[i].1.min(right[j].1);

        if start <= end {
            total += u64::from(end) - u64::from(start) + 1;
        }

        if left[i].1 < right[j].1 {
            i += 1;
        } else {
            j += 1;
        }
    }

    total
}

fn total_addresses(intervals: &[Interval]) -> u64 {
    intervals
        .iter()
        .map(|(start, end)| u64::from(*end) - u64::from(*start) + 1)
        .sum()
}

fn ipv4_to_u32(address: Ipv4Addr) -> u32 {
    u32::from(address)
}

fn percentage(part: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        part as f64 * 100.0 / total as f64
    }
}
