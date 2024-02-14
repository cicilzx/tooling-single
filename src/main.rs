#![feature(rustc_private)]

extern crate rustc_ast_pretty;
extern crate rustc_driver;
extern crate rustc_error_codes;
extern crate rustc_errors;
extern crate rustc_hash;
extern crate rustc_hir;
extern crate rustc_interface;
extern crate rustc_session;
extern crate rustc_span;
extern crate rustc_middle;

use std::{any::Any, path, process, str::{self, FromStr}, sync::Arc};
use rustc_middle::{hir::nested_filter, query::Key, ty::{Ty, TyCtxt, TyKind}};
use rustc_ast_pretty::pprust::item_to_string;
use rustc_errors::registry;
use rustc_session::config;
use rustc_errors::DIAGNOSTICS;
use std::path::PathBuf;
use rustc_hir::{intravisit::Visitor, Stmt};
use rustc_span::source_map::SourceMap;
use rustc_hir::{Expr, Item, def::Res};


pub fn get_config(input_path: PathBuf) -> rustc_interface::Config {
    let out = process::Command::new("rustc")
        .arg("--print=sysroot")
        .current_dir(".")
        .output()
        .unwrap();
    let sysroot = str::from_utf8(&out.stdout).unwrap().trim();
    let config = rustc_interface::Config {
        opts: config::Options {
            maybe_sysroot: Some(path::PathBuf::from(sysroot)),
            ..config::Options::default()
        },
        input: config::Input::File(input_path.clone()),
        crate_cfg: Vec::new(),
        crate_check_cfg: Vec::new(),
        output_dir: None,
        output_file: None,
        file_loader: None,
        locale_resources: rustc_driver::DEFAULT_LOCALE_RESOURCES,
        lint_caps: rustc_hash::FxHashMap::default(),
        parse_sess_created: None,
        register_lints: None,
        override_queries: None,
        make_codegen_backend: None,
        registry: registry::Registry::new(DIAGNOSTICS),
        expanded_args: Vec::new(),
        ice_file: None,
        hash_untracked_state: None,
        using_internal_features: Arc::default(),
    };
    config
}

#[derive(Debug)]
pub struct VarInfo<'tcx> {
    name: String,
    start_line: usize,
    start_col: usize,
    start_file: Option<PathBuf>,
    end_line: usize,
    end_col: usize,
    end_file: Option<PathBuf>,
    ty: Option<Ty<'tcx>>,
}

fn extract_local_path(name: &rustc_span::FileName) -> Option<PathBuf> {
    if let rustc_span::FileName::Real(f) = name {
        if let rustc_span::RealFileName::LocalPath(p) = f {
            Some(p.clone())
        } else {
            None
        }
    } else {
        None
    }
}



struct HirVisitor<'tcx> {
    tcx: TyCtxt<'tcx>,
}

struct All;

impl<'hir> rustc_hir::intravisit::nested_filter::NestedFilter<'hir> for All {
    type Map = rustc_middle::hir::map::Map<'hir>;
    const INTER: bool = true;
    const INTRA: bool = true;
}


impl<'tcx> rustc_hir::intravisit::Visitor<'tcx> for HirVisitor<'tcx> {
    type Map = rustc_middle::hir::map::Map<'tcx>;
    type NestedFilter = All;

    fn nested_visit_map(&mut self) -> Self::Map {
        self.tcx.hir()
    }


    fn visit_local(&mut self, local: &'tcx rustc_hir::Local<'tcx>) {
        let source_map = self.tcx.sess.source_map();
        let ident_name = local.pat.simple_ident().unwrap().name.as_str().to_string();

        // println!("{:#?}", var_span);
        let var_span = local.pat.span.data();
        let start = source_map.lookup_char_pos(var_span.lo);
        let end = source_map.lookup_char_pos(var_span.hi);

        let start_path = extract_local_path(&start.file.name);
        let end_path = extract_local_path(&end.file.name);

        let ty = if let Some(expr) = local.init {
            let hir_id = expr.hir_id;
            let def_id = hir_id.owner.def_id;
            let ty = self.tcx.typeck(def_id).node_type(hir_id);
            Some(ty)
        } else {
            None
        };

        let var_info = VarInfo {
            name: ident_name,
            start_line: start.line,
            start_col: start.col_display,
            end_line: end.line,
            end_col: end.col_display,
            ty,
            start_file: start_path,
            end_file: end_path,
        };
        println!("{:#?}", var_info);
        rustc_hir::intravisit::walk_local(self, local);
    }

    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        let source_map = self.tcx.sess.source_map();

        if let rustc_hir::ExprKind::Path(qpath) = expr.kind {
            if let rustc_hir::QPath::Resolved(_, p) = qpath {

                for seg in p.segments.into_iter() {
                    if let rustc_hir::def::Res::Local(hir_id) = p.res {
                        let def_id = hir_id.owner.def_id;
                        let ty = self.tcx.typeck(def_id).node_type(hir_id);
                        let ident_name = seg.ident.name.as_str().to_string();
                        let var_span = seg.ident.span.data();
                        let start = source_map.lookup_char_pos(var_span.lo);
                        let end = source_map.lookup_char_pos(var_span.hi);

                        let start_path = extract_local_path(&start.file.name);
                        let end_path = extract_local_path(&end.file.name);

                        let var_info = VarInfo {
                            name: ident_name,
                            start_line: start.line,
                            start_col: start.col_display,
                            end_line: end.line,
                            end_col: end.col_display,
                            ty: Some(ty),
                            start_file: start_path,
                            end_file: end_path,
                        };
                        println!("{:#?}", var_info);
                    }
                }

            }
        }
        //println!("{:#?}", expr);
        rustc_hir::intravisit::walk_expr(self, expr);
    }
}

fn main() {
    let file_path = PathBuf::from_str("./src/t.rs").unwrap();
    let config = get_config(file_path);

    rustc_interface::run_compiler(config, |compiler| {
        compiler.enter(|queries| {
            // Analyze the crate and inspect the types under the cursor.
            queries.global_ctxt().unwrap().enter(|tcx| {
                let hir_krate = tcx.hir();
                //let source_map = tcx.sess.source_map();
                //parse(tcx);

                let mut visitor = HirVisitor {
                    tcx,
                };

                
                for id in hir_krate.items() {
                    // let item = hir_krate.item(id);
                    rustc_hir::intravisit::Visitor::visit_nested_item(&mut visitor, id);


                }
                
            })
        });
    });
}
