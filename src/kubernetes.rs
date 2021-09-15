use crate::config::KubeConfig;
use crate::errors::KubernetesError;
use base64;
use isahc::{
    config::CaCertificate, config::ClientCertificate, config::Configurable, config::PrivateKey,
    config::SslOption, HttpClient, Request, Body
};
use http::StatusCode;
use k8s_openapi::{ResponseBody, api::core::v1 as api};
use std::{io::Read, io::Write};
use tempfile::NamedTempFile;
use chrono::DateTime;
use std::env;

#[derive(Debug)]
pub struct Kubernetes {
    pub kubeconfig: Result<KubeConfig, KubernetesError>,
    pub http_client: HttpClient,
    pub base_uri: String,
}

impl Kubernetes {
    pub fn connect(kubeconfig_path: Option<String>, scheme: Option<String>, host: Option<String>, port: Option<u32>, search_uri: bool) -> Result<Kubernetes, KubernetesError> {
        let kubeconfig = KubeConfig::load(kubeconfig_path);
        let http_client;
        if let Ok(conf) = &kubeconfig {
            //TODO add options, guessed from config
            if let Some(cluster) = conf.clusters.first() {
                if let Some(auth_info) = conf.auth_infos.first() {
                    let user = &auth_info.auth_info;
                    if let Some(crt) = &user.client_certificate_data {
                        if let Some(ca) = &cluster.cluster.certificate_authority_data {
                            if let Some(key) = &user.client_key_data {
                                let mut tmpfile = NamedTempFile::new()
                                    .map_err(|err| KubernetesError::IoError { source: err })?;
                                writeln!(tmpfile, "{}", ca)
                                    .map_err(|err| KubernetesError::IoError { source: err })?;
                                let http_client_builder = HttpClient::builder()
                                    .ssl_client_certificate(ClientCertificate::pem(
                                        base64::decode(crt).map_err(|err| {
                                            KubernetesError::Base64DecodeError { source: err }
                                        })?,
                                        PrivateKey::pem(
                                            base64::decode(key).map_err(|err| {
                                                KubernetesError::Base64DecodeError { source: err }
                                            })?,
                                            None,
                                        ),
                                    ))
                                    .ssl_ca_certificate(CaCertificate::file(
                                        tmpfile.into_temp_path().to_path_buf(),
                                    ))
                                    .ssl_options(SslOption::DANGER_ACCEPT_INVALID_CERTS);
                                http_client = match http_client_builder.build() {
                                    Ok(client) => client,
                                    Err(err) => return Err(KubernetesError::HttpClientBuildError { message: format!("Failed to initialize http client with client certificate: {}", err) })
                                };
                            } else {
                                return Err(KubernetesError::HttpClientBuildError {
                                    message: String::from(
                                        "Couldn't get client key from kubeconfig.",
                                    ),
                                });
                            }
                        } else {
                            return Err(KubernetesError::HttpClientBuildError {
                                message: String::from(
                                    "Couldn't get CA certificate from kubeconfig.",
                                ),
                            });
                        }
                    } else {
                        return Err(KubernetesError::HttpClientBuildError {
                            message: String::from("Couldn't get client private key."),
                        });
                    }
                } else {
                    return Err(KubernetesError::HttpClientBuildError {
                        message: String::from("No auth_info item found in kubeconfig."),
                    });
                }
            } else {
                return Err(KubernetesError::ConfigLoadError);
            }
        } else {
            return Err(KubernetesError::HttpClientBuildError {
                message: String::from("Couldn't gather kubeconfig content."),
            });
        }

        let scheme_part;
        let host_part;
        let port_part;

        if search_uri {
            if let Ok(host_var) = env::var("KUBERNETES_SERVICE_HOST") {
                host_part = host_var;
            } else {
                eprintln!("Couldn't determine kubernetes service host from environment.");
                host_part = host.unwrap_or(String::from("localhost"));
            }
            if let Ok(port_var) = env::var("KUBERNETES_SERVICE_PORT") {
                port_part = port_var;
                scheme_part = match port_part.as_str() {
                    "443" => String::from("https"),
                    "80" => String::from("http"),
                    _ => String::from("https")
                }
            } else {
                scheme_part = String::from("https");
                port_part = port.unwrap_or(6443).to_string();
                eprintln!("Couldn't determine kubernetes services port from environment.");
            }
        } else {
            scheme_part = scheme.unwrap_or(String::from("https"));
            host_part = host.unwrap_or(String::from("localhost"));
            port_part = port.unwrap_or(6443).to_string();
        }

        let base_uri = format!("{}://{}:{}", scheme_part, host_part, port_part);

        Ok(Kubernetes {
            kubeconfig,
            http_client,
            base_uri
        })
    }

    fn request<T>(&self, request: Request<Vec<u8>>, response_body: fn(StatusCode)->ResponseBody<T>) -> Result<(Body, ResponseBody<T>), KubernetesError>{
        let (parts, body) = request.into_parts();
        let uri_str = format!("{}{}", self.base_uri, parts.uri);
        let request = Request::builder().uri(uri_str).body(body).map_err(|err| {
            KubernetesError::HttpClientBuildError {
                message: format!("Couldn't build request. Error: {:?}", err),
            }
        })?;
        let response = self.http_client.send(request).map_err(|_| KubernetesError::HttpClientRequestError)?;
        println!("Got the response: {:?}", response);
        let status_code = response.status();
        if !status_code.is_success() {
            return Err(KubernetesError::HttpClientRequestError);
        }
        let response_body = response_body(status_code);
        let body = response.into_body();
        Ok((body, response_body))
    }

    pub fn get_events(&self, since: Option<String>) -> Result<Vec<api::Event>, KubernetesError> {
        let (request, response_body) =
            match api::Event::list_event_for_all_namespaces(Default::default()) {
                Ok((request, response_body)) => (request, response_body),
                Err(err) => return Err(KubernetesError::ApiRequestError { source: err }),
            };
        let (mut body, mut response_body) = self.request(request, response_body)?;
        let mut buf = Box::new([0u8; 4096]);
        let events_list_raw = loop {
            let read = body.read(&mut * buf).map_err(|err| {
                KubernetesError::HttpClientParseResponseError {
                    message: format!("Got error: {}", err),
                }
            })?;
            response_body.append_slice(&buf[..read]);
            let response = response_body.parse();
            match response {
                Ok(k8s_openapi::ListResponse::Ok(events_list)) => break events_list,
                Ok(other) => {
                    return Err(KubernetesError::HttpClientParseResponseError {
                        message: format!("expected Ok but got {:?}", other),
                    })
                }
                Err(k8s_openapi::ResponseError::NeedMoreData) => continue,
                Err(err) => {
                    return Err(KubernetesError::HttpClientParseResponseError {
                        message: format!("error: {:?}", err),
                    })
                }
            }
        };
        let events = events_list_raw.items;
        let mut since_datetime = None;
        if let Some(since) = since {
            since_datetime = Some(DateTime::parse_from_rfc3339(&since).map_err(|source| KubernetesError::WrongDatetimeFormat{ source })?);
        }
        Ok(
            events.into_iter().filter(
                move |e| {
                    match &e.event_time {
                        Some(time) => {
                            if let Some(since_dt) = since_datetime {
                                if time.0.ge(&since_dt){
                                    return true
                                } else {
                                    return false
                                }
                            } else {
                                return true
                            }
                        },
                        None => false
                    }
                }
            ).collect()
        )
    }

    pub fn list_pods(&self, namespace: String) -> Result<Vec<api::Pod>, KubernetesError> {
        let (request, response_body) =
            match api::Pod::list_namespaced_pod(&namespace, Default::default()) {
                Ok((request, response_body)) => (request, response_body),
                Err(err) => return Err(KubernetesError::ApiRequestError { source: err }),
            };
        let (parts, body) = request.into_parts();
        let uri_str = format!("{}{}", self.base_uri, parts.uri);
        let request = Request::builder().uri(uri_str).body(body).map_err(|err| {
            KubernetesError::HttpClientBuildError {
                message: format!("Couldn't build request. Error: {:?}", err),
            }
        })?;
        let response = self
            .http_client
            .send(request)
            .map_err(|_| KubernetesError::HttpClientRequestError)?;
        println!("Got the response: {:?}", response);
        let status_code = response.status();
        if !status_code.is_success() {
            return Err(KubernetesError::HttpClientRequestError);
        }
        let mut response_body = response_body(status_code);
        let mut buf = Box::new([0u8; 4096]);
        let mut body = response.into_body();
        let pods_list_raw = loop {
            let read = body.read(&mut *buf).map_err(|err| {
                KubernetesError::HttpClientParseResponseError {
                    message: format!("Got error : {}", err),
                }
            })?;
            response_body.append_slice(&buf[..read]);
            let response = response_body.parse();
            match response {
                Ok(k8s_openapi::ListResponse::Ok(pod_list)) => break pod_list,
                Ok(other) => {
                    return Err(KubernetesError::HttpClientParseResponseError {
                        message: format!("expected Ok but got {} {:?}", status_code, other),
                    })
                }
                Err(k8s_openapi::ResponseError::NeedMoreData) => continue,
                Err(err) => {
                    return Err(KubernetesError::HttpClientParseResponseError {
                        message: format!("error: {} {:?}", status_code, err),
                    })
                }
            }
        };

        Ok(pods_list_raw.items)
    }
}
