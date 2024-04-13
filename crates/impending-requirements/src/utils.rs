pub fn pep508_normalize(pkgname: &str) -> String {
    // @TODO: Ask Claude for this
    // @TODO: Do PEP 508, lol
    return pkgname.to_lowercase().replace("-", "_");
}
