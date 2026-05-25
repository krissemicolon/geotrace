use curl::easy::Easy;
use serde::Deserialize;

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
    let mut easy = Easy::new();
    let mut body = Vec::new();
    let url = format!("http://ip-api.com/json/{host}?fields=continentCode,country,city,lat,lon");

    easy.url(&url)
        .map_err(|e| format!("failed to set geo url: {e}"))?;

    {
        let mut transfer = easy.transfer();
        transfer
            .write_function(|data| {
                body.extend_from_slice(data);
                Ok(data.len())
            })
            .map_err(|e| format!("failed to set write callback: {e}"))?;

        transfer
            .perform()
            .map_err(|e| format!("geo request failed: {e}"))?;
    }

    let response: IpApiGeoResponse =
        serde_json::from_slice(&body).map_err(|e| format!("invalid json in geo response: {e}"))?;

    Ok(GeoInfo {
        continent_code: response.continent_code,
        country: response.country,
        city: response.city,
        coord: (response.lon, response.lat),
    })
}
