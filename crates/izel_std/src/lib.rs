pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }

    #[test]
    fn std_iter_exposes_full_combinator_surface() {
        let iter_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../library/std/iter.iz");
        let src = fs::read_to_string(&iter_path)
            .unwrap_or_else(|e| panic!("failed to read {:?}: {}", iter_path, e));

        let required = [
            "forge map<",
            "forge filter<",
            "forge filter_map<",
            "forge flat_map<",
            "forge flatten<",
            "forge fold<",
            "forge scan<",
            "forge take<",
            "forge skip<",
            "forge take_while<",
            "forge skip_while<",
            "forge zip<",
            "forge enumerate<",
            "forge chain<",
            "forge peekable<",
            "forge cloned<",
            "forge collect<",
            "forge count<",
            "forge sum<",
            "forge product<",
            "forge min<",
            "forge max<",
            "forge any<",
            "forge all<",
            "forge find<",
            "forge position<",
            "forge partition<",
            "forge chunks<",
            "forge windows<",
        ];

        for symbol in required {
            assert!(
                src.contains(symbol),
                "missing std::iter combinator declaration: {}",
                symbol
            );
        }
    }
}
