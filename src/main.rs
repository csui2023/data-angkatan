use std::{collections::HashMap, env, fs, io::Write, sync::Arc};

use convert_case::{Case, Casing};
use image::EncodableLayout;
use reqwest::header::HeaderMap;
use serde::Serialize;

#[derive(Serialize)]
struct Mahasiswa<'a> {
    id: &'a str,
    #[serde(rename(serialize = "namaLengkap"))]
    nama_lengkap: &'a str,
    #[serde(rename(serialize = "namaPanggilan"))]
    nama_panggilan: &'a str,
    jurusan: &'a str,
    tanggal_lahir: &'a str,
    linkedin: &'a str,
    instagram: &'a str,
    twitter: &'a str,
    line: &'a str,
    deskripsi: &'a str,
    message: &'a str,
    interests: Vec<&'a str>,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().expect(".env file not found");

    let mut headers = HeaderMap::new();
    headers.insert(
        "X-goog-api-key",
        env::var("GOOGLE_API_KEY").unwrap().parse().unwrap(),
    );

    let client = Arc::new(
        reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap(),
    );

    let result = client
        .get(format!(
            "https://www.googleapis.com/drive/v3/files/{}/export",
            env::var("SHEETS_FILE_ID").unwrap()
        ))
        .query(&[("mimeType", "text/csv")])
        .send()
        .await
        .unwrap();

    fs::create_dir("./dist").unwrap();
    let result = result.text().await.unwrap();
    let mut rdr = csv::Reader::from_reader(result.as_bytes());
    let mut futs = Vec::new();

    for record in rdr.records() {
        let record = record.unwrap();
        let client = Arc::clone(&client);

        futs.push(async move {
            let mahasiswa = Mahasiswa {
                id: &record[2].to_string().to_case(Case::Kebab),
                nama_lengkap: &record[2],
                nama_panggilan: &record[3],
                jurusan: &record[6],
                tanggal_lahir: &record[7],
                linkedin: &record[8],
                instagram: &record[9],
                twitter: &record[10],
                line: &record[11],
                deskripsi: &record[12],
                message: &record[13],
                interests: vec![&record[14], &record[15], &record[16]],
            };

            let stringified = serde_json::to_string_pretty(&mahasiswa).unwrap();
            let filename = format!("./dist/{}.json", mahasiswa.id);
            let mut file = fs::File::create(filename).unwrap();
            file.write_all(stringified.as_bytes()).unwrap();

            let pfp_url = reqwest::Url::parse(&record[5]).unwrap();
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

            let img = image::load_from_memory(profile_picture.as_bytes()).unwrap();
            let img = img.resize(512, 512, image::imageops::FilterType::Lanczos3);
            let filename = format!("./dist/{}.png", mahasiswa.id);
            img.save(filename).unwrap();

            println!("Saved {}.", mahasiswa.id);
        })
    }

    futures::future::join_all(futs).await;
}
