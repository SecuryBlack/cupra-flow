//! Tráfico por dominio para el panel (fase 2): cada minuto lee los contadores
//! de Caddy (`/metrics`, con `per_host`), calcula lo que han subido desde la
//! lectura anterior y lo manda por OTLP a Nexus (`127.0.0.1:4317`), igual
//! que OxiPulse. edge-gateway lo escribe en `proxy_metrics`.
//!
//! Por dominio y minuto: el histograma de duración (`count` son las
//! peticiones) y las respuestas por clase (2xx…5xx).

use opentelemetry_proto::tonic::collector::metrics::v1::{
    metrics_service_client::MetricsServiceClient, ExportMetricsServiceRequest,
};
use opentelemetry_proto::tonic::common::v1::{any_value, AnyValue, InstrumentationScope, KeyValue};
use opentelemetry_proto::tonic::metrics::v1::{
    metric, number_data_point, AggregationTemporality, Histogram, HistogramDataPoint, Metric,
    NumberDataPoint, ResourceMetrics, ScopeMetrics, Sum,
};
use opentelemetry_proto::tonic::resource::v1::Resource;
use std::collections::HashMap;

/// Los cubos por defecto de Caddy, en segundos (más `+Inf`).
pub const BOUNDS: [f64; 11] = [
    0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
];
const BUCKETS: usize = BOUNDS.len() + 1;
/// Caddy mide cada manejador; cada petición acaba en uno solo de estos. Así
/// `vars` y `subroute` (rutas con caminos o IPs permitidas) no cuentan doble.
const TERMINAL: [&str; 2] = ["reverse_proxy", "static_response"];
const NEXUS: &str = "http://127.0.0.1:4317";

/// Contadores acumulados de un dominio: cubos `le` (acumulativos, como los
/// da Prometheus) y respuestas por clase (índice = código / 100).
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct Counters {
    pub buckets: [u64; BUCKETS],
    pub classes: [u64; 6],
}

impl Counters {
    pub fn requests(&self) -> u64 {
        self.buckets[BUCKETS - 1]
    }
}

fn labels(s: &str) -> HashMap<&str, &str> {
    // Los valores (hosts, códigos, nombres de manejador) no llevan `",`.
    s.split("\",")
        .filter_map(|kv| {
            let (k, v) = kv.split_once("=\"")?;
            Some((k.trim_start_matches(',').trim(), v.trim_end_matches('"')))
        })
        .collect()
}

/// Suma por dominio las series del servidor de SB.
pub fn parse(text: &str) -> HashMap<String, Counters> {
    let mut out: HashMap<String, Counters> = HashMap::new();
    for line in text.lines() {
        let Some(rest) = line.strip_prefix("caddy_http_request_duration_seconds_") else {
            continue;
        };
        let Some((kind, rest)) = rest.split_once('{') else {
            continue;
        };
        let Some((lab, val)) = rest.rsplit_once("} ") else {
            continue;
        };
        let l = labels(lab);
        if l.get("server") != Some(&crate::caddy::SERVER)
            || !l.get("handler").is_some_and(|h| TERMINAL.contains(h))
        {
            continue;
        }
        let (Some(host), Ok(v)) = (l.get("host"), val.trim().parse::<f64>()) else {
            continue;
        };
        let v = v as u64;
        match kind {
            "bucket" => {
                let i = match l.get("le") {
                    Some(&"+Inf") => BUCKETS - 1,
                    Some(le) => match le
                        .parse::<f64>()
                        .ok()
                        .and_then(|x| BOUNDS.iter().position(|b| *b == x))
                    {
                        Some(i) => i,
                        None => continue,
                    },
                    None => continue,
                };
                out.entry(host.to_string()).or_default().buckets[i] += v;
            }
            "count" => {
                let class = l
                    .get("code")
                    .and_then(|c| c.chars().next())
                    .and_then(|c| c.to_digit(10))
                    .unwrap_or(0)
                    .min(5) as usize;
                out.entry(host.to_string()).or_default().classes[class] += v;
            }
            _ => {}
        }
    }
    out
}

/// Lo que ha subido cada dominio desde `prev`. Si un contador baja, Caddy se
/// reinició: cuenta lo que lleva desde entonces. Solo dominios con tráfico.
pub fn delta(
    prev: &HashMap<String, Counters>,
    cur: &HashMap<String, Counters>,
) -> HashMap<String, Counters> {
    cur.iter()
        .filter_map(|(host, c)| {
            let p = prev.get(host).copied().unwrap_or_default();
            let reset = c.buckets.iter().zip(p.buckets).any(|(a, b)| *a < b)
                || c.classes.iter().zip(p.classes).any(|(a, b)| *a < b);
            let mut d = *c;
            if !reset {
                for (a, b) in d.buckets.iter_mut().zip(p.buckets) {
                    *a -= b;
                }
                for (a, b) in d.classes.iter_mut().zip(p.classes) {
                    *a -= b;
                }
            }
            (d.requests() > 0).then(|| (host.clone(), d))
        })
        .collect()
}

fn kv(key: &str, value: &str) -> KeyValue {
    KeyValue {
        key: key.into(),
        value: Some(AnyValue {
            value: Some(any_value::Value::StringValue(value.into())),
        }),
        ..Default::default()
    }
}

/// Una petición OTLP con el minuto de cada dominio (`start`..`now`, en ns).
pub fn request(d: &HashMap<String, Counters>, start: u64, now: u64) -> ExportMetricsServiceRequest {
    let delta = AggregationTemporality::Delta as i32;
    let hist = d
        .iter()
        .map(|(host, c)| HistogramDataPoint {
            attributes: vec![kv("host", host)],
            start_time_unix_nano: start,
            time_unix_nano: now,
            count: c.requests(),
            // OTLP quiere cada cubo por separado, no acumulado.
            bucket_counts: {
                let mut last = 0;
                c.buckets
                    .iter()
                    .map(|b| {
                        let b = (*b).max(last);
                        let n = b - last;
                        last = b;
                        n
                    })
                    .collect()
            },
            explicit_bounds: BOUNDS.to_vec(),
            ..Default::default()
        })
        .collect();
    let classes = d
        .iter()
        .flat_map(|(host, c)| {
            c.classes
                .iter()
                .enumerate()
                .filter(|(_, n)| **n > 0)
                .map(move |(i, n)| NumberDataPoint {
                    attributes: vec![
                        kv("host", host),
                        kv("http.response.status_class", &format!("{i}xx")),
                    ],
                    start_time_unix_nano: start,
                    time_unix_nano: now,
                    value: Some(number_data_point::Value::AsInt(*n as i64)),
                    ..Default::default()
                })
        })
        .collect();
    ExportMetricsServiceRequest {
        resource_metrics: vec![ResourceMetrics {
            resource: Some(Resource {
                attributes: vec![
                    kv("service.name", "cupraflow"),
                    kv("service.version", env!("CARGO_PKG_VERSION")),
                ],
                ..Default::default()
            }),
            scope_metrics: vec![ScopeMetrics {
                scope: Some(InstrumentationScope {
                    name: "cupraflow".into(),
                    ..Default::default()
                }),
                metrics: vec![
                    Metric {
                        name: "http.server.request.duration".into(),
                        unit: "s".into(),
                        data: Some(metric::Data::Histogram(Histogram {
                            data_points: hist,
                            aggregation_temporality: delta,
                        })),
                        ..Default::default()
                    },
                    Metric {
                        name: "http.server.responses".into(),
                        data: Some(metric::Data::Sum(Sum {
                            data_points: classes,
                            aggregation_temporality: delta,
                            is_monotonic: true,
                        })),
                        ..Default::default()
                    },
                ],
                ..Default::default()
            }],
            ..Default::default()
        }],
    }
}

/// Cuándo arrancó el proceso de Caddy, para saber si se reinició.
pub fn process_start(text: &str) -> Option<f64> {
    text.lines()
        .find_map(|l| l.strip_prefix("process_start_time_seconds "))
        .and_then(|v| v.trim().parse().ok())
}

fn now_ns() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

/// Lo que se recuerda entre minutos: la última lectura y cuándo fue.
#[derive(Default)]
pub struct State {
    last: Option<(u64, HashMap<String, Counters>)>,
    /// `process_start_time_seconds` de Caddy en la última lectura.
    started: Option<f64>,
    warned: bool,
}

/// Una vuelta: con Caddy en marcha, lee, calcula y manda. La primera lectura
/// solo sirve de base.
// ponytail: si Nexus no responde, ese minuto se pierde (no hay cola). Nexus es
// local y casi siempre está; añadir una cola como la de OxiPulse si hace falta.
pub async fn tick(st: &mut State) {
    if let Err(e) = crate::caddy::ensure_metrics().await {
        tracing::debug!("metrics: cannot enable per-host metrics: {e}");
    }
    let (cur, started) = match crate::caddy::scrape().await {
        Ok(text) => (parse(&text), process_start(&text)),
        Err(e) => {
            tracing::debug!("metrics: scrape failed: {e}");
            return;
        }
    };
    let now = now_ns();
    let Some((start, mut prev)) = st.last.replace((now, cur.clone())) else {
        st.started = started;
        return;
    };
    // Caddy se reinició: todo lo que lleva cuenta desde cero, aunque un
    // contador ya haya superado al de antes.
    if started.is_some() && std::mem::replace(&mut st.started, started) != started {
        prev.clear();
    }
    let d = delta(&prev, &cur);
    if d.is_empty() {
        return;
    }
    // Con un plazo: un Nexus colgado no puede parar el bucle de `main`.
    let sent = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        let channel = tonic::transport::Channel::from_static(NEXUS)
            .connect()
            .await?;
        MetricsServiceClient::new(channel)
            .export(request(&d, start, now))
            .await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    })
    .await
    .unwrap_or_else(|_| Err("timed out after 10 s".into()));
    match sent {
        Ok(()) => st.warned = false,
        Err(e) if !st.warned => {
            st.warned = true;
            tracing::warn!("metrics: Nexus did not take the traffic metrics: {e}");
        }
        Err(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"# HELP caddy_http_request_duration_seconds Histogram
caddy_http_request_duration_seconds_bucket{code="200",handler="reverse_proxy",host="a.com",method="GET",server="sb",le="0.005"} 1
caddy_http_request_duration_seconds_bucket{code="200",handler="reverse_proxy",host="a.com",method="GET",server="sb",le="0.01"} 2
caddy_http_request_duration_seconds_bucket{code="200",handler="reverse_proxy",host="a.com",method="GET",server="sb",le="+Inf"} 6
caddy_http_request_duration_seconds_count{code="200",handler="reverse_proxy",host="a.com",method="GET",server="sb"} 6
caddy_http_request_duration_seconds_bucket{code="502",handler="reverse_proxy",host="a.com",method="GET",server="sb",le="+Inf"} 2
caddy_http_request_duration_seconds_count{code="502",handler="reverse_proxy",host="a.com",method="GET",server="sb"} 2
caddy_http_request_duration_seconds_count{code="200",handler="vars",host="a.com",method="GET",server="sb"} 8
caddy_http_request_duration_seconds_count{code="200",handler="subroute",host="a.com",method="GET",server="sb"} 8
caddy_http_request_duration_seconds_count{code="308",handler="static_response",host="a.com",method="GET",server="remaining_auto_https_redirects"} 3
caddy_http_request_duration_seconds_count{code="301",handler="static_response",host="b.com",method="GET",server="sb"} 4
caddy_http_request_duration_seconds_bucket{code="301",handler="static_response",host="b.com",method="GET",server="sb",le="+Inf"} 4
caddy_http_requests_total{handler="reverse_proxy",host="a.com",server="sb"} 8
"#;

    #[test]
    fn parses_terminal_handlers_of_the_sb_server() {
        let m = parse(SAMPLE);
        let a = m["a.com"];
        assert_eq!(a.requests(), 8);
        assert_eq!(a.buckets[0], 1);
        assert_eq!(a.buckets[1], 2);
        assert_eq!(a.classes[2], 6);
        assert_eq!(a.classes[5], 2);
        assert_eq!(m["b.com"].classes[3], 4);
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn delta_subtracts_and_survives_a_restart() {
        let cur = parse(SAMPLE);
        let mut prev = cur.clone();
        prev.get_mut("a.com").unwrap().buckets[BUCKETS - 1] = 5;
        prev.get_mut("a.com").unwrap().classes[2] = 3;
        let d = delta(&prev, &cur);
        assert_eq!(d["a.com"].requests(), 3);
        assert_eq!(d["a.com"].classes[2], 3);
        assert!(!d.contains_key("b.com"), "no traffic, no row");

        let mut higher = cur.clone();
        higher.get_mut("b.com").unwrap().buckets[BUCKETS - 1] = 100;
        let d = delta(&higher, &cur);
        assert_eq!(
            d["b.com"].requests(),
            4,
            "counter went down: Caddy restarted"
        );
    }

    #[test]
    fn reads_process_start() {
        let text = "# TYPE process_start_time_seconds gauge
process_start_time_seconds 1.79476e+09
";
        assert_eq!(process_start(text), Some(1.79476e9));
        assert_eq!(process_start(SAMPLE), None);
    }

    #[test]
    fn request_has_separate_buckets_and_classes() {
        let d = parse(SAMPLE);
        let r = request(&d, 1, 2);
        let m = &r.resource_metrics[0].scope_metrics[0].metrics;
        let Some(metric::Data::Histogram(h)) = &m[0].data else {
            panic!()
        };
        let a = h
            .data_points
            .iter()
            .find(|p| p.attributes[0].value == kv("host", "a.com").value)
            .unwrap();
        assert_eq!(a.count, 8);
        assert_eq!(a.bucket_counts.iter().sum::<u64>(), 8);
        assert_eq!(&a.bucket_counts[..3], &[1, 1, 0]);
        let Some(metric::Data::Sum(s)) = &m[1].data else {
            panic!()
        };
        assert_eq!(s.data_points.len(), 3);
    }
}
