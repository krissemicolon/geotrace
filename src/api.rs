use curl::easy::Easy;

pub type Coord = (f64, f64);

#[derive(Debug, Clone)]
pub struct GeoInfo {
    pub continent_code: String,
    pub country: String,
    pub coord: Coord,
}

pub fn get_geo_from_host(host: &str) -> Result<GeoInfo, String> {
    let mut easy = Easy::new();
    let mut body = Vec::new();
    let url = format!("http://ip-api.com/line/{host}?fields=continentCode,country,lat,lon");

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

    let text = String::from_utf8(body).map_err(|e| format!("invalid utf8 in geo response: {e}"))?;
    let mut lines = text.lines();

    let continent_code = lines
        .next()
        .ok_or_else(|| "missing continent code in geo response".to_string())?
        .trim()
        .to_string();

    let country = lines
        .next()
        .ok_or_else(|| "missing country in geo response".to_string())?
        .trim()
        .to_string();

    let lat: f64 = lines
        .next()
        .ok_or_else(|| "missing latitude in geo response".to_string())?
        .trim()
        .parse()
        .map_err(|e| format!("invalid latitude in geo response: {e}"))?;

    let lon: f64 = lines
        .next()
        .ok_or_else(|| "missing longitude in geo response".to_string())?
        .trim()
        .parse()
        .map_err(|e| format!("invalid longitude in geo response: {e}"))?;

    Ok(GeoInfo {
        continent_code,
        country,
        coord: (lon, lat),
    })
}
