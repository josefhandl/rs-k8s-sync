use k8s_sync;

#[test]
fn read_config() {
    let client = k8s_sync::kubernetes::Kubernetes::connect(
        Some(String::from("tests/fixtures/kubeconfig")),
        None,
        None,
        None,
        false,
    );
    assert!(client.is_ok());
    let client = client.unwrap();
    assert!(client.kubeconfig.is_ok());
    let kubeconfig = client.kubeconfig.unwrap();
    assert!(!kubeconfig.auth_infos.is_empty())
}

#[test]
fn list_pods() {
    let client = k8s_sync::kubernetes::Kubernetes::connect(
        Some(String::from("/root/.kube/config")),
        None,
        None,
        None,
        false,
    );
    println!("Connecting to cluter");
    if let Ok(c) = client {
        let pods = c.list_pods(String::from("kube-system"));
        assert!(pods.is_ok());
        let pods_raw = pods.unwrap();
        if !pods_raw.is_empty() {
            let first = pods_raw.first().unwrap();
            println!("POD : {:?}", first);
        }
    } else {
        eprintln!("Couldn't connect to cluster");
    }
}
