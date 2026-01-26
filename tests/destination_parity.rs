use rand_core::OsRng;

#[test]
fn destination_hash_matches_python() {
    let identity = reticulum::identity::PrivateIdentity::new_from_rand(OsRng);
    let dest = reticulum::destination::new_in(identity, "app", "aspect");
    assert_eq!(dest.desc.address_hash.len(), 16);
}
