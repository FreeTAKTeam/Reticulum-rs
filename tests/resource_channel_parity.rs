#[test]
fn resource_state_machine_advances() {
    let mut res = reticulum::resource::Resource::new();
    res.begin();
    assert!(res.is_active());
}
