use regex::Regex;

#[derive(Debug)]
pub enum TarFileTy {
    Update(String),
    Delete(String),
}

impl From<String> for TarFileTy {
    fn from(val: String) -> Self {
        if let Ok(valid_ident) = Regex::new(r"(.*?/?)(\.wh\.)([^/]+)$") {
            if valid_ident.is_match(val.as_str()) {
                Self::Delete(valid_ident.replace(val.as_str(), "$1$3").to_string())
            } else {
                Self::Update(val)
            }
        } else {
            unreachable!()
        }
    }
}

#[cfg(test)]
mod test {
    use regex::Regex;
    #[test]
    fn test_regex() {
        /// "Cargo.toml"
        // ".wh.dir"
        // "dir/.wh.Cargo.toml.copy"
        let valid_ident: Regex = Regex::new(r"(.*?)/?(\.wh\.)?([^/]+)$").unwrap();
        {
            let res = valid_ident.captures("Cargo.toml").unwrap();
            assert_eq!(res.get(1).unwrap().as_str(), "");
            assert!(res.get(2).is_none());
            assert_eq!(res.get(3).unwrap().as_str(), "Cargo.toml");
        }
        {
            let res = valid_ident.captures(".wh.dir").unwrap();
            assert_eq!(res.get(1).unwrap().as_str(), "");
            assert_eq!(res.get(2).unwrap().as_str(), ".wh.");
            assert_eq!(res.get(3).unwrap().as_str(), "dir");
        }
        {
            let res = valid_ident.captures("dir/.wh.Cargo.toml.copy").unwrap();
            assert_eq!(res.get(1).unwrap().as_str(), "dir");
            assert_eq!(res.get(2).unwrap().as_str(), ".wh.");
            assert_eq!(res.get(3).unwrap().as_str(), "Cargo.toml.copy");
        }
        {
            let res = valid_ident.captures("dir/dir2/Cargo.toml.copy").unwrap();
            assert_eq!(res.get(1).unwrap().as_str(), "dir/dir2");
            assert!(res.get(2).is_none());
            assert_eq!(res.get(3).unwrap().as_str(), "Cargo.toml.copy");
        }
    }
    #[test]
    fn test_regex2() {
        /// "Cargo.toml"
        // ".wh.dir"
        // "dir/.wh.Cargo.toml.copy"
        let valid_ident: Regex = Regex::new(r"(.*?/?)(\.wh\.)([^/]+)$").unwrap();
        {
            assert_eq!(valid_ident.is_match("Cargo.toml"), false);
            assert_eq!(valid_ident.is_match(".wh.dir"), true);
            assert_eq!(valid_ident.is_match("dir/.wh.Cargo.toml.copy"), true);
            assert_eq!(valid_ident.is_match("dir/dir2/Cargo.toml.copy"), false);
            assert_eq!(valid_ident.is_match("dir/dir2/.wh.Cargo.toml.copy"), true);

            assert_eq!(valid_ident.replace("Cargo.toml", "$1$3"), "Cargo.toml");
            assert_eq!(valid_ident.replace(".wh.dir", "$1$3"), "dir");
            assert_eq!(
                valid_ident.replace("dir/.wh.Cargo.toml.copy", "$1$3"),
                "dir/Cargo.toml.copy"
            );
            assert_eq!(
                valid_ident.replace("dir/dir2/Cargo.toml.copy", "$1$3"),
                "dir/dir2/Cargo.toml.copy"
            );
            assert_eq!(
                valid_ident.replace("dir/dir2/.wh.Cargo.toml.copy", "$1$3"),
                "dir/dir2/Cargo.toml.copy"
            );
        }
    }
}
