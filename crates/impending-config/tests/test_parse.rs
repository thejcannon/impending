#[cfg(test)]
pub mod tests {

    use impending_config::PyProject;

    #[test]
    fn test_empty() {
        toml::from_str::<PyProject>("").unwrap();
    }

    #[test]
    fn test_unrelated() {
        toml::from_str::<PyProject>("x=1").unwrap();
        toml::from_str::<PyProject>("[tool.black]\nline-length=88").unwrap();
    }

    #[test]
    fn test_empty_table() {
        let config = toml::from_str::<PyProject>("[tool.impending]").unwrap();
        let config = config.tool.unwrap().impending.unwrap();
        assert_eq!(config.lockfile, None);
        assert_eq!(config.installer_cmd, None);
    }
}
