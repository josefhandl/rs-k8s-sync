use k8s_sync;

#[test]
fn connect_to_cluster() {
    let client = k8s_sync::kubernetes::Kubernetes::connect(Some(String::from("tests/fixtures/kubeconfig")));
    assert!(client.is_ok());
    let client = client.unwrap();
    assert!(client.kubeconfig.is_ok());
    assert!(!client.kubeconfig.unwrap().auth_infos.is_empty())
}