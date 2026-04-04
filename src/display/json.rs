pub fn display_report_json<T: serde::Serialize>(report: &T) {
    match serde_json::to_string_pretty(report) {
        Ok(json) => println!("{}", json),
        Err(e) => eprintln!("Error serializing report to JSON: {}", e),
    }
}
