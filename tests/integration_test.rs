use k8s_sync::{self, config::KubeConfig};

#[test]
fn read_config() {
    let client = k8s_sync::kubernetes::Kubernetes::connect(Some(String::from("tests/fixtures/kubeconfig")));
    assert!(client.is_ok());
    let client = client.unwrap();
    assert!(client.kubeconfig.is_ok());
    let kubeconfig = client.kubeconfig.unwrap();
    assert!(!kubeconfig.auth_infos.is_empty())
}

#[test]
fn list_pods() {
    let client = k8s_sync::kubernetes::Kubernetes::connect(Some(String::from("/root/.kube/config")));
    println!("Connecting to cluter");
    if let Ok(c) = client {
        let pods = c.list_pods(String::from("kube-system"));
        assert!(pods.is_ok());
        let pods_raw = pods.unwrap();
        println!("POD : {:?}", pods_raw.first().unwrap());
        //for p in pods_raw {
        //    println!("POD : {}", p);
        //}
    } else {
        eprintln!("Couldn't connect to cluster");
    }
}