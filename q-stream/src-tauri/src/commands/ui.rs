/// Extract dominant color from an image URL for dynamic background
#[tauri::command]
pub async fn extract_dominant_color(image_url: String) -> Result<[u8; 3], String> {
    if image_url.is_empty() {
        return Ok([18, 18, 24]); // Default dark background
    }

    // Handle base64 data URLs (local files)
    let image_bytes = if image_url.starts_with("data:") {
        let parts: Vec<&str> = image_url.splitn(2, ",").collect();
        if parts.len() != 2 {
            return Ok([18, 18, 24]);
        }
        use base64::Engine;
        base64::engine::general_purpose::STANDARD
            .decode(parts[1])
            .map_err(|e| format!("Failed to decode base64: {}", e))?
    } else {
        // Download the image
        let client = reqwest::Client::new();
        let resp = client
            .get(&image_url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch image: {}", e))?;
        resp.bytes()
            .await
            .map_err(|e| format!("Failed to read image bytes: {}", e))?
            .to_vec()
    };

    // Parse the image
    let img = image::load_from_memory(&image_bytes)
        .map_err(|e| format!("Failed to parse image: {}", e))?;

    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();

    // Simple dominant color extraction via averaging
    // Sample center region to avoid edges
    let x_start = width / 4;
    let x_end = 3 * width / 4;
    let y_start = height / 4;
    let y_end = 3 * height / 4;

    let mut r_sum: u64 = 0;
    let mut g_sum: u64 = 0;
    let mut b_sum: u64 = 0;
    let mut count: u64 = 0;

    for y in y_start..y_end {
        for x in x_start..x_end {
            let pixel = rgba.get_pixel(x, y);
            r_sum += pixel[0] as u64;
            g_sum += pixel[1] as u64;
            b_sum += pixel[2] as u64;
            count += 1;
        }
    }

    if count == 0 {
        return Ok([18, 18, 24]);
    }

    // Darken the color for background use (multiply by 0.3)
    let r = ((r_sum / count) as f64 * 0.3) as u8;
    let g = ((g_sum / count) as f64 * 0.3) as u8;
    let b = ((b_sum / count) as f64 * 0.3) as u8;

    Ok([r, g, b])
}
