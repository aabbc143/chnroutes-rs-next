use chnroutes::{Result, Source, Target};

fn main() -> Result<()> {
    // Get the CN IP routes from APNIC.
    let cn_ip_results = chnroutes::source::apnic::fetch_ip_data()?;

    println!("Loaded {} CN IP networks.", cn_ip_results.len());

    // Generate the Linux routing script.
    let (up_script, down_script) = Target::Linux.export_str(&Source::Apnic)?;

    println!("\n--- UP SCRIPT ---\n{up_script}");
    println!("\n--- DOWN SCRIPT ---\n");

    if let Some(down_script) = down_script {
        println!("{down_script}");
    }

    Ok(())
}
