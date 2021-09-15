use k8s_openapi::RequestError;

#[derive(Debug)]
pub enum KubernetesError {
    RequestError,
    IoError { source: std::io::Error },
    ClientBuildError,
    HttpClientBuildError { message: String },
    HttpClientRequestError,
    HttpClientParseResponseError { message: String },
    ApiRequestError { source: RequestError },
    Base64DecodeError { source: base64::DecodeError },
    InvalidDataError,
    ConfigLoadError,
    WrongDatetimeFormat { source : chrono::ParseError }
}

impl std::error::Error for KubernetesError {}

impl std::fmt::Display for KubernetesError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            KubernetesError::RequestError => write!(f, "Request Error (k8s_openapi)"),
            KubernetesError::IoError { source } => {
                write!(f, "Couldn't read file (io::Error). Source: {}", source)
            }
            KubernetesError::ClientBuildError => write!(f, "Couldn't build client (isahc)"),
            KubernetesError::HttpClientBuildError { message } => {
                write!(f, "Couldn't build http client (isahc): {}", message)
            }
            KubernetesError::HttpClientRequestError => {
                write!(f, "Couldn't build http client (isahc)")
            }
            KubernetesError::HttpClientParseResponseError { message } => {
                write!(f, "Couldn't parse response from HTTP server: {}", message)
            }
            KubernetesError::Base64DecodeError { source } => {
                write!(f, "Couldn't decode base 64. Source: {}", source)
            }
            KubernetesError::InvalidDataError => write!(f, "Invalid data provided."),
            KubernetesError::ConfigLoadError => write!(f, "Could not load Kube Config."),
            KubernetesError::ApiRequestError { source } => {
                write!(f, "API returned error: {}.", source)
            },
            KubernetesError::WrongDatetimeFormat { source } => {
                write!(f, "Couldn't parse date time input : {}", source)
            }
        }
    }
}
