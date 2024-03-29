use crate::{DepMap, ReqInfo, ReqMap};
use anyhow::Result;
use std::collections::HashMap;

pub fn parse_requirements_txt(path: &str) -> Result<(ReqMap, DepMap)> {
    let contents = std::fs::read_to_string(path)?;
    let code = contents.as_bytes();

    let mut reqmap = HashMap::new();
    let mut depmap = HashMap::new();

    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(tree_sitter_requirements::language())
        .expect("Error loading requirements grammar");
    let tree = parser.parse(code, None).unwrap();
    let mut cursor = tree.walk();
    cursor.goto_first_child();

    loop {
        let node = cursor.node();
        if node.kind() == "requirement" {
            cursor.goto_first_child();
            let child = cursor.node();
            assert_eq!(child.kind(), "package");
            let pkgname = child.utf8_text(code).unwrap().to_owned();
            let mut req_info = ReqInfo::new();
            loop {
                if !cursor.goto_next_sibling() {
                    break;
                }
                let child = cursor.node();
                let contents = child.utf8_text(code).unwrap().to_string();

                match child.kind() {
                    "extras" => req_info.extras = Some(contents),
                    "version_spec" => req_info.version_spec = Some(contents),
                    "url_spec" => req_info.url_spec = Some(contents),
                    "marker_spec" => req_info.marker_spec = Some(contents),
                    "requirement_opts" => {
                        match req_info.opts {
                            Some(ref mut opts) => opts.push_str(&contents),
                            None => req_info.opts = Some(contents),
                        };
                    }
                    _ => (),
                }
            }
            reqmap.insert(pkgname.clone(), req_info);
            cursor.goto_parent();
            if !cursor.goto_next_sibling() {
                break;
            }
            if cursor.node().kind() == "comment" {
                let contents = cursor.node().utf8_text(code).unwrap();
                if let Some(comment) = contents.strip_prefix("# via") {
                    if comment.is_empty() {
                        // NB: --annotation-style split
                        loop {
                            if !cursor.goto_next_sibling() {
                                break;
                            }
                            if cursor.node().kind() == "comment" {
                                let rdep_name = cursor
                                    .node()
                                    .utf8_text(code)
                                    .unwrap()
                                    .strip_prefix('#')
                                    .unwrap()
                                    .trim();
                                depmap
                                    .entry(rdep_name.to_owned())
                                    .or_insert_with(Vec::new)
                                    .push(pkgname.clone());
                            } else {
                                break;
                            }
                        }
                    } else {
                        // NB: --annotation-style line
                        let pkgs: Vec<String> =
                            comment.split(',').map(|s| s.trim().to_string()).collect();
                        for rdep_name in pkgs {
                            depmap
                                .entry(rdep_name)
                                .or_insert_with(Vec::new)
                                .push(pkgname.clone());
                        }
                    }
                }
            }
        } else if !cursor.goto_next_sibling() {
            break;
        }
    }

    Ok((reqmap, depmap))
}
