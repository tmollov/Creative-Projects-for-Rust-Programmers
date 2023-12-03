// Test it with the following commands:
// curl -X DELETE http://localhost:8080/datafile.txt
// curl -X GET http://localhost:8080/datafile.txt
// curl -X PUT http://localhost:8080/datafile.txt -d "File contents."
// curl -X POST http://localhost:8080/data -d "File contents."
// curl -X GET http://localhost:8080/a/b
//
// after running the second command, the client should have printed:
// Contents of the file.
//
// After running all five commands, the server should have printed:
// Listening at address 127.0.0.1:8080 ...
// Deleting file "datafile.txt" ... Deleted file "datafile.txt"
// Downloading file "datafile.txt" ... Downloaded file "datafile.txt"
// Uploading file "datafile.txt" ... Uploaded file "datafile.txt"
// Uploading file "data_*.txt" ... Uploaded file "data_17.txt"
// Invalid URI: "/a/b"

use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use futures_util::StreamExt;
use std::io::Write;

fn flush_stdout() {
    std::io::stdout().flush().unwrap();
}

async fn delete_file(info: web::Path<String>) -> impl Responder {
    let filename = &info;
    println!("Deleting file \"{}\" ... ", filename);
    flush_stdout();

    let file = std::fs::remove_file(filename.to_string());
    match file {
        Ok(_ok) => {
            println!("Deleted file \"{}\"", filename);
            return HttpResponse::Ok();
        }
        Err(err) => {
            println!("Error while deleting file \"{}\": {}", filename, err);
            return HttpResponse::NotFound();
        }
    }
}

async fn download_file(info: web::Path<String>) -> impl Responder {
    let filename = &info;
    println!("Downloading file \"{}\" ... ", filename);
    flush_stdout();

    let contents = std::fs::read_to_string(filename.to_string());

    match contents {
        Ok(data) => {
            println!("Downloaded file \"{}\"", filename);
            return HttpResponse::Ok().content_type("text/plain").body(data);
        }
        Err(err) => {
            println!("Error downloading the file: {}", err);
            return HttpResponse::NotFound().into();
        }
    }
}

async fn upload_specified_file(
    info: web::Path<String>,
    mut payload: web::Payload,
) -> impl Responder {
    let filename = &info;
    print!("Uploading file \"{}\" ... ", filename);
    flush_stdout();

    // Read the request payload (file contents)
    let mut contents = web::BytesMut::new();
    while let Some(chunk) = payload.next().await {
        contents.extend_from_slice(&chunk.unwrap());
    }

    let contents_str = String::from_utf8_lossy(&contents);
    std::fs::write(filename.to_string(), &*contents_str).unwrap();

    println!("Uploaded file \"{}\"", filename);
    HttpResponse::Ok()
}

async fn upload_new_file(info: web::Path<String>, mut payload: web::Payload) -> impl Responder {
    let filename = &info;
    print!("Uploading file \"{}*.txt\" ... ", filename);
    flush_stdout();

    let mut contents = web::BytesMut::new();
    while let Some(chunk) = payload.next().await {
        contents.extend_from_slice(&chunk.unwrap());
    }

    let contents_str = String::from_utf8_lossy(&contents);
    let file_id = 17;
    let filename = format!("{}{}.txt", filename, file_id);
    std::fs::write(filename.to_string(), &*contents_str).unwrap();

    println!("Uploaded file \"{}\"", filename);
    HttpResponse::Ok().content_type("text/plain").body(filename)
}

async fn invalid_resource(req: HttpRequest) -> impl Responder {
    println!("Invalid URI: \"{}\"", req.uri());
    HttpResponse::NotFound()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let server_address = "127.0.0.1:8080";
    println!("Listening at address {} ...", server_address);
    HttpServer::new(|| {
        App::new()
            .service(
                web::resource("/{filename}")
                    .route(web::delete().to(delete_file))
                    .route(web::get().to(download_file))
                    .route(web::put().to(upload_specified_file))
                    .route(web::post().to(upload_new_file)),
            )
            .default_service(web::route().to(invalid_resource))
    })
    .bind(server_address)?
    .run()
    .await
}

