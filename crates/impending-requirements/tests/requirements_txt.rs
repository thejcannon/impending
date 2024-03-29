#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use impending_requirements::{parse_requirements_txt, ReqInfo};

    fn load_file(path: &str) -> String {
        let mut resource_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        resource_path.push("tests/lockfiles/");
        resource_path.push(path);

        let contents = fs::read_to_string(resource_path).expect("Failed to read file");
        contents
    }

    #[test]
    fn parsing_simple() {
        for req_txt in [
            "requirements_no_annotate.txt",
            "requirements_line.txt",
            "requirements_split.txt",
        ] {
            let (reqmap, depmap) = parse_requirements_txt(load_file(req_txt).as_bytes());
            assert_eq!(reqmap.len(), 9);

            for (pkgname, pkgversion) in [
                ("certifi", "==2024.2.2"),
                ("charset-normalizer", "==3.3.2"),
                ("idna", "==3.6"),
                ("pydantic", "==2.6.1"),
                ("pydantic-core", "==2.16.2"),
                ("requests", "==2.31.0"),
                ("typing-extensions", "==4.9.0"),
                ("urllib3", "==2.2.1"),
            ] {
                assert_eq!(
                    reqmap
                        .get(pkgname)
                        .expect(format!("Expected {}", pkgname).as_str()),
                    &ReqInfo {
                        extras: None,
                        version_spec: Some(pkgversion.to_owned()),
                        url_spec: None,
                        marker_spec: None,
                        opts: None,
                    }
                );
            }

            if req_txt == "requirements_no_annotate.txt" {
                assert_eq!(depmap.len(), 0);
            } else {
                println!("{:?}", depmap);
                assert_eq!(depmap.len(), 4);
                assert_eq!(
                    depmap.get("requests").unwrap(),
                    &vec!["certifi", "charset-normalizer", "idna", "urllib3"]
                );
                assert_eq!(
                    depmap.get("pydantic").unwrap(),
                    &vec!["annotated-types", "pydantic-core", "typing-extensions"]
                );
                assert_eq!(
                    depmap.get("pydantic-core").unwrap(),
                    &vec!["typing-extensions"]
                );
            }
        }
    }
}
