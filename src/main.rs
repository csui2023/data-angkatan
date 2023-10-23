use std::{collections::HashMap, env, fs, io::Write};

use convert_case::{Case, Casing};
use image::{EncodableLayout, GenericImageView};
use regex::Regex;
use reqwest::header::HeaderMap;
use serde::Serialize;

#[derive(Serialize)]
struct Mahasiswa {
    id: String,
    #[serde(rename(serialize = "namaLengkap"))]
    nama_lengkap: String,
    #[serde(rename(serialize = "namaPanggilan"))]
    nama_panggilan: String,
    jurusan: String,
    tanggal_lahir: String,
    linkedin: String,
    instagram: String,
    twitter: String,
    line: String,
    deskripsi: String,
    message: String,
    interests: Vec<String>,
    #[serde(skip_serializing)]
    pfp_file_id: String,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().expect(".env file not found");

    let mut headers = HeaderMap::new();
    headers.insert(
        "X-goog-api-key",
        env::var("GOOGLE_API_KEY").unwrap().parse().unwrap(),
    );

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .unwrap();

    let result = client
        .get(format!(
            "https://www.googleapis.com/drive/v3/files/{}/export",
            env::var("SHEETS_FILE_ID").unwrap()
        ))
        .query(&[("mimeType", "text/csv")])
        .send()
        .await
        .unwrap();

    fs::create_dir_all("./data/pfp").unwrap();
    let result = result.text().await.unwrap();
    let mut rdr = csv::Reader::from_reader(result.as_bytes());
    let mut data: Vec<Mahasiswa> = Vec::new();

    for record in rdr.records() {
        let record = record.unwrap();

        let re = Regex::new(r"[^A-Za-z ]").unwrap();
        let id = re.replace_all(&record[2].to_string(), "").to_string();
        let id = id.to_case(Case::Kebab);

        let mahasiswa = Mahasiswa {
            id: id.clone(),
            nama_lengkap: record[2].to_string(),
            nama_panggilan: record[3].to_string(),
            jurusan: record[6].to_string(),
            tanggal_lahir: record[7].to_string(),
            linkedin: record[8].to_string(),
            instagram: record[9].to_string(),
            twitter: record[10].to_string(),
            line: record[11].to_string(),
            deskripsi: record[12].to_string(),
            message: record[13].to_string(),
            interests: vec![
                record[14].to_string(),
                record[15].to_string(),
                record[16].to_string(),
            ],
            pfp_file_id: record[5].to_string(),
        };

        data.push(mahasiswa);
    }

    data.sort_by(|a, b| a.nama_lengkap.cmp(&b.nama_lengkap));
    let stringified = serde_json::to_string_pretty(&data).unwrap();
    let mut file = fs::File::create("./data/data.json").unwrap();
    file.write_all(stringified.as_bytes()).unwrap();

    for mhs in data {
        let filename = format!("./data/pfp/{}.png", mhs.id);
        if fs::metadata(&filename).is_ok() {
            println!("Skipping {}. Already exist.", mhs.id);
            continue;
        }

        let pfp_url = reqwest::Url::parse(&mhs.pfp_file_id).unwrap();
        let pair: HashMap<_, String> = pfp_url.query_pairs().into_owned().collect();
        let url = format!(
            "https://www.googleapis.com/drive/v3/files/{}",
            pair.get("id").unwrap()
        );

        let profile_picture = client
            .get(url)
            .query(&[("alt", "media")])
            .send()
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();

        let bytes = profile_picture.as_bytes();

        match image::load_from_memory(bytes) {
            Ok(img) => {
                let (x, y) = img.dimensions();

                let img = if x < y {
                    img.crop_imm(0, (y - x) / 2, x, x)
                } else {
                    img.crop_imm((x - y) / 2, 0, y, y)
                };

                let img = img.resize(512, 512, image::imageops::FilterType::Lanczos3);
                img.save(filename).unwrap();

                println!("Saved {}.", mhs.id);
            }
            Err(err) => {
                println!("Error {}. {}.", mhs.id, err.to_string());
                // break;
            }
        }
    }
}
