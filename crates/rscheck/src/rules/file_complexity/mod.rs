use crate::analysis::Workspace;
use crate::config::{ComplexityMode, FileComplexityConfig};
use crate::emit::Emitter;
use crate::report::{FileMetrics, Finding};
use crate::rules::{Rule, RuleBackend, RuleContext, RuleFamily, RuleInfo};
use crate::span::{Location, Span};
use std::path::Path;
use syn::visit::Visit;

pub struct FileComplexityRule;

impl FileComplexityRule {
    pub fn static_info() -> RuleInfo {
        RuleInfo {
            id: "shape.file_complexity",
            family: RuleFamily::Shape,
            backend: RuleBackend::Syntax,
            summary: "Measures file and function complexity against configured limits.",
            default_level: FileComplexityConfig::default().level,
            schema: "level, mode, max_file, max_fn, count_question, match_arms",
            config_example: "[rules.\"shape.file_complexity\"]\nlevel = \"warn\"\nmode = \"cyclomatic\"\nmax_file = 200\nmax_fn = 25",
            fixable: false,
        }
    }
}

impl Rule for FileComplexityRule {
    fn info(&self) -> RuleInfo {
        Self::static_info()
    }

    fn run(&self, ws: &Workspace, ctx: &RuleContext<'_>, out: &mut dyn Emitter) {
        for file in &ws.files {
            let cfg = match ctx
                .policy
                .decode_rule::<FileComplexityConfig>(Self::static_info().id, Some(&file.path))
            {
                Ok(cfg) => cfg,
                Err(_) => continue,
            };
            let Some(ast) = &file.ast else { continue };

            match cfg.mode {
                ComplexityMode::Cyclomatic => {
                    let mut v = CyclomaticVisitor {
                        count_question: cfg.count_question,
                        match_arms: cfg.match_arms,
                        per_fn: Vec::new(),
                    };
                    v.visit_file(ast);

                    let sum = v.per_fn.iter().map(|c| c.score).sum::<u32>();
                    let max_fn = v.per_fn.iter().map(|c| c.score).max().unwrap_or(0);

                    out.record_metrics(FileMetrics {
                        path: file.path.to_string_lossy().to_string(),
                        cyclomatic_sum: sum,
                        cyclomatic_max_fn: max_fn,
                    });

                    let over_file = sum > cfg.max_file;
                    let over_fn = max_fn > cfg.max_fn;
                    if over_file || over_fn {
                        let mut msg = String::new();
                        if over_file {
                            msg.push_str(&format!(
                                "file cyclomatic complexity sum {sum} exceeds {}\n",
                                cfg.max_file
                            ));
                        }
                        if over_fn {
                            msg.push_str(&format!(
                                "max function cyclomatic complexity {max_fn} exceeds {}\n",
                                cfg.max_fn
                            ));
                        }
                        out.emit(Finding {
                            rule_id: Self::static_info().id.to_string(),
                            family: Some(Self::static_info().family),
                            engine: Some(Self::static_info().backend),
                            severity: cfg.level.to_severity(),
                            message: msg.trim_end().to_string(),
                            primary: Some(file_span(&file.path)),
                            secondary: Vec::new(),
                            help: Some(
                                "Split the functions or module to reduce branching.".to_string(),
                            ),
                            evidence: Some(format_per_fn(&v.per_fn)),
                            confidence: None,
                            tags: vec!["complexity".to_string()],
                            labels: Vec::new(),
                            notes: Vec::new(),
                            fixes: Vec::new(),
                        });
                    }
                }
                ComplexityMode::PhysicalLoc => {
                    let loc = count_physical_loc(&file.text);
                    if loc as u32 > cfg.max_file {
                        out.emit(Finding {
                            rule_id: Self::static_info().id.to_string(),
                            family: Some(Self::static_info().family),
                            engine: Some(Self::static_info().backend),
                            severity: cfg.level.to_severity(),
                            message: format!("file physical LOC {loc} exceeds {}", cfg.max_file),
                            primary: Some(file_span(&file.path)),
                            secondary: Vec::new(),
                            help: Some("Split the file into smaller modules.".to_string()),
                            evidence: None,
                            confidence: None,
                            tags: vec!["size".to_string()],
                            labels: Vec::new(),
                            notes: Vec::new(),
                            fixes: Vec::new(),
                        });
                    }
                }
                ComplexityMode::LogicalLoc => {
                    let mut v = LogicalLocVisitor { stmts: 0 };
                    v.visit_file(ast);
                    let ll = v.stmts;
                    if ll > cfg.max_file {
                        out.emit(Finding {
                            rule_id: Self::static_info().id.to_string(),
                            family: Some(Self::static_info().family),
                            engine: Some(Self::static_info().backend),
                            severity: cfg.level.to_severity(),
                            message: format!("file logical LOC {ll} exceeds {}", cfg.max_file),
                            primary: Some(file_span(&file.path)),
                            secondary: Vec::new(),
                            help: Some("Split the file into smaller modules.".to_string()),
                            evidence: None,
                            confidence: None,
                            tags: vec!["size".to_string()],
                            labels: Vec::new(),
                            notes: Vec::new(),
                            fixes: Vec::new(),
                        });
                    }
                }
            }
        }
    }
}

fn file_span(path: &Path) -> Span {
    Span::new(
        path,
        Location { line: 1, column: 1 },
        Location { line: 1, column: 1 },
    )
}

fn format_per_fn(per_fn: &[FnScore]) -> String {
    let mut out = String::new();
    for s in per_fn {
        out.push_str(&format!("{}: {}\n", s.name, s.score));
    }
    out
}

fn count_physical_loc(text: &str) -> usize {
    text.lines()
        .filter_map(|line| {
            let t = line.trim();
            if t.is_empty() {
                return None;
            }
            if t.starts_with("//") {
                return None;
            }
            Some(())
        })
        .count()
}

#[derive(Debug, Clone)]
struct FnScore {
    name: String,
    score: u32,
}

struct CyclomaticVisitor {
    count_question: bool,
    match_arms: bool,
    per_fn: Vec<FnScore>,
}

impl CyclomaticVisitor {
    fn bump(&mut self, n: u32) {
        if let Some(last) = self.per_fn.last_mut() {
            last.score = last.score.saturating_add(n);
        }
    }
}

impl<'ast> Visit<'ast> for CyclomaticVisitor {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        let name = node.sig.ident.to_string();
        self.per_fn.push(FnScore { name, score: 1 });
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        let name = node.sig.ident.to_string();
        self.per_fn.push(FnScore { name, score: 1 });
        syn::visit::visit_impl_item_fn(self, node);
    }

    fn visit_expr_if(&mut self, node: &'ast syn::ExprIf) {
        self.bump(1);
        syn::visit::visit_expr_if(self, node);
    }

    fn visit_expr_for_loop(&mut self, node: &'ast syn::ExprForLoop) {
        self.bump(1);
        syn::visit::visit_expr_for_loop(self, node);
    }

    fn visit_expr_while(&mut self, node: &'ast syn::ExprWhile) {
        self.bump(1);
        syn::visit::visit_expr_while(self, node);
    }

    fn visit_expr_loop(&mut self, node: &'ast syn::ExprLoop) {
        self.bump(1);
        syn::visit::visit_expr_loop(self, node);
    }

    fn visit_expr_match(&mut self, node: &'ast syn::ExprMatch) {
        if self.match_arms {
            self.bump(node.arms.len() as u32);
        } else {
            self.bump(1);
        }
        syn::visit::visit_expr_match(self, node);
    }

    fn visit_expr_binary(&mut self, node: &'ast syn::ExprBinary) {
        if matches!(node.op, syn::BinOp::And(_) | syn::BinOp::Or(_)) {
            self.bump(1);
        }
        syn::visit::visit_expr_binary(self, node);
    }

    fn visit_expr_try(&mut self, node: &'ast syn::ExprTry) {
        if self.count_question {
            self.bump(1);
        }
        syn::visit::visit_expr_try(self, node);
    }
}

struct LogicalLocVisitor {
    stmts: u32,
}

impl<'ast> Visit<'ast> for LogicalLocVisitor {
    fn visit_stmt(&mut self, node: &'ast syn::Stmt) {
        self.stmts = self.stmts.saturating_add(1);
        syn::visit::visit_stmt(self, node);
    }
}

#[cfg(test)]
mod tests;
