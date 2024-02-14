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

use std::{path, process, str::{self, FromStr}, sync::Arc};
use rustc_middle::ty::{Ty, TyCtxt, TyKind};
use rustc_ast_pretty::pprust::item_to_string;
use rustc_errors::registry;
use rustc_session::config;
use rustc_errors::DIAGNOSTICS;
use std::path::PathBuf;
use rustc_hir::Stmt;
use rustc_span::source_map::SourceMap;

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

pub fn parse_local<'tcx>(tcx: TyCtxt<'tcx>, stmt:&Stmt, item:&rustc_hir::Item, source_map: &SourceMap) -> Option<VarInfo<'tcx>> {
    println!("{:#?}", stmt);

    if let rustc_hir::StmtKind::Local(local) = stmt.kind {
        let ident_name = local.pat.simple_ident().unwrap().name.as_str().to_string();

        // println!("{:#?}", var_span);
        let var_span = local.pat.span.data();
        let start = source_map.lookup_char_pos(var_span.lo);
        let end = source_map.lookup_char_pos(var_span.hi);

        let start_path = extract_local_path(&start.file.name);
        let end_path = extract_local_path(&end.file.name);

        let ty = if let Some(expr) = local.init {
            let hir_id = expr.hir_id;
            let def_id = item.hir_id().owner.def_id;
            let ty = tcx.typeck(def_id).node_type(hir_id);
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
        Some(var_info)
    } else {
        // println!("stmt kind: {:#?}", stmt.kind);
        None
    }
}


pub fn parse<'tcx>(tcx: TyCtxt<'tcx>) {
    // Every compilation contains a single crate.
    let hir_krate = tcx.hir();
    let source_map = tcx.sess.source_map();
    
    for id in hir_krate.items() {
        let item = hir_krate.item(id);
        // processing the functions
        if let rustc_hir::ItemKind::Fn(_, _, body_id) = item.kind {
            let fn_body_expr = &tcx.hir().body(body_id).value;
            // Function body is a block expr
            if let rustc_hir::ExprKind::Block(block, _) = fn_body_expr.kind {
                for stmt in block.stmts.into_iter() {
                    let var_info = parse_local(tcx, stmt, item, source_map);
                    //println!("{:#?}", var_info);
                }
            }
        }
    }
}

fn main() {
    let file_path = PathBuf::from_str("./src/t.rs").unwrap();
    let config = get_config(file_path);

    rustc_interface::run_compiler(config, |compiler| {
        compiler.enter(|queries| {
            // Analyze the crate and inspect the types under the cursor.
            queries.global_ctxt().unwrap().enter(|tcx| {
                parse(tcx);
            })
        });
    });
}
