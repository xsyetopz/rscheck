use rscheck::report::{Report, Severity};

pub fn to_html(report: &Report) -> String {
    let mut html = String::new();
    html.push_str("<!doctype html><html><head><meta charset=\"utf-8\"/>");
    html.push_str("<title>rscheck report</title>");
    html.push_str(
        "<style>body{font-family:ui-sans-serif,system-ui,Segoe UI,Roboto,Helvetica,Arial}\
         table{border-collapse:collapse;width:100%}th,td{border:1px solid #ddd;padding:6px}\
         th{text-align:left;background:#f6f6f6}\
         .sev-info{color:#555}.sev-warn{color:#b45309}.sev-deny{color:#b91c1c}\
         code,pre{font-family:ui-monospace,SFMono-Regular,Menlo,Monaco,Consolas,monospace}\
         </style>",
    );
    html.push_str("</head><body>");
    html.push_str("<h1>rscheck report</h1>");
    html.push_str("<table><thead><tr><th>Severity</th><th>Rule</th><th>Location</th><th>Message</th></tr></thead><tbody>");

    for f in &report.findings {
        let (sev_class, sev) = match f.severity {
            Severity::Info => ("sev-info", "info"),
            Severity::Warn => ("sev-warn", "warn"),
            Severity::Deny => ("sev-deny", "deny"),
        };
        let loc = f.primary.as_ref().map_or_else(
            || "".to_string(),
            |s| format!("{}:{}:{}", s.file, s.start.line, s.start.column),
        );
        html.push_str(&format!(
            "<tr><td class=\"{sev_class}\">{sev}</td><td><code>{}</code></td><td><code>{loc}</code></td><td>{}</td></tr>",
            escape_html(&f.rule_id),
            escape_html(&f.message)
        ));
        if let Some(evidence) = &f.evidence {
            html.push_str(&format!(
                "<tr><td colspan=\"4\"><pre>{}</pre></td></tr>",
                escape_html(evidence)
            ));
        }
    }

    html.push_str("</tbody></table></body></html>");
    html
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
