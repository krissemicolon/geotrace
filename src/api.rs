use serde::Deserialize;
use std::io::{Read, Write};
use std::net::TcpStream;

pub type Coord = (f64, f64);

#[derive(Debug, Clone)]
pub struct GeoInfo {
    pub continent_code: String,
    pub country: String,
    pub city: String,
    pub coord: Coord,
}

#[derive(Debug, Deserialize)]
struct IpApiGeoResponse {
    #[serde(rename = "continentCode")]
    continent_code: String,
    country: String,
    city: String,
    lat: f64,
    lon: f64,
}

pub fn lookup_geo_info(host: &str) -> Result<GeoInfo, String> {
    let mut stream = TcpStream::connect(("ip-api.com", 80))
        .map_err(|e| format!("failed to connect to geo service: {e}"))?;

    let request = format!(
        "GET /json/{host}?fields=continentCode,country,city,lat,lon HTTP/1.1\r\nHost: ip-api.com\r\nConnection: close\r\nUser-Agent: geotrace\r\nAccept: application/json\r\n\r\n"
    );

    stream
        .write_all(request.as_bytes())
        .map_err(|e| format!("failed to send geo request: {e}"))?;

    let mut response_buf = Vec::new();
    stream
        .read_to_end(&mut response_buf)
        .map_err(|e| format!("failed to read geo response: {e}"))?;

    let header_end = response_buf
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or_else(|| "invalid http response from geo service".to_string())?;

    let (headers, body) = response_buf.split_at(header_end + 4);

    let status_line_end = headers
        .windows(2)
        .position(|w| w == b"\r\n")
        .ok_or_else(|| "invalid status line from geo service".to_string())?;

    let status_line = std::str::from_utf8(&headers[..status_line_end])
        .map_err(|e| format!("invalid status line encoding: {e}"))?;

    if !status_line.contains(" 200 ") {
        return Err(format!("geo request failed with status: {status_line}"));
    }

    let response: IpApiGeoResponse =
        serde_json::from_slice(body).map_err(|e| format!("invalid json in geo response: {e}"))?;

    Ok(GeoInfo {
        continent_code: response.continent_code,
        country: response.country,
        city: response.city,
        coord: (response.lon, response.lat),
    })
}
