fn main() {
    let path = "../../fixtures/demo/snapshot.json";
    println!("cargo:rerun-if-changed={path}");
    let value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("Démonstration manquante"))
            .unwrap();
    for d in value["documents"].as_array().unwrap() {
        assert!(
            d["id"].as_str().unwrap().starts_with("demo-"),
            "Interdit d'embarquer des notes réelles dans le site"
        );
        assert_eq!(d["original"], "");
    }
    for q in value["questions"].as_array().unwrap() {
        for c in q["sources"].as_array().unwrap() {
            assert!(c["document_id"].as_str().unwrap().starts_with("demo-"));
        }
    }
}
