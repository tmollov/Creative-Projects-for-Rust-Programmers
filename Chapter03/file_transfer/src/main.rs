// Test it with the following commands:
// curl -X DELETE http://localhost:8080/datafile.txt
// curl -X GET http://localhost:8080/datafile.txt
// curl -X PUT http://localhost:8080/datafile.txt -d "File contents."
// curl -X POST http://localhost:8080/data -d "File contents."
// curl -X GET http://localhost:8080/a/b

use actix_web::Error;
use actix_web::{web, web::Path, App, HttpRequest, HttpResponse, HttpServer, Responder};
use futures::{TryFutureExt, TryStreamExt};
use rand::prelude::*;
use std::fs::File;
use std::io::Write;
use tokio::fs::File as TokioFile;
use tokio::io::AsyncWriteExt;

fn flush_stdout() {
    std::io::stdout().flush().unwrap();
}

async fn delete_file(info: Path<(String,)>) -> impl Responder {
    let filename = &info.0;
    print!("Deleting file \"{}\" ... ", filename);
    flush_stdout();

    // Delete the file.
    match std::fs::remove_file(&filename) {
        Ok(_) => {
            println!("Deleted file \"{}\"", filename);
            HttpResponse::Ok()
        }
        Err(error) => {
            println!("Failed to delete file \"{}\": {}", filename, error);
            HttpResponse::NotFound()
        }
    }
}

async fn download_file(info: Path<(String,)>) -> impl Responder {
    let filename = &info.0;
    print!("Downloading file \"{}\" ... ", filename);
    flush_stdout();

    fn read_file_contents(filename: &str) -> std::io::Result<String> {
        use std::io::Read;
        let mut contents = String::new();
        File::open(filename)?.read_to_string(&mut contents)?;
        Ok(contents)
    }

    match read_file_contents(&filename) {
        Ok(contents) => {
            println!("Downloaded file \"{}\"", filename);
            HttpResponse::Ok().content_type("text/plain").body(contents)
        }
        Err(error) => {
            println!("Failed to read file \"{}\": {}", filename, error);
            HttpResponse::NotFound().finish()
        }
    }
}

async fn upload_specified_file(
    payload: web::Payload,
    info: web::Path<(String,)>,
) -> impl Responder {
    let filename = info.0.clone();

    print!("Uploading file \"{}\" ... ", filename);
    flush_stdout();

    // Get asynchronously from the client
    // the contents to write into the file.
    payload
        .map_err(Error::from)
        .try_fold(web::BytesMut::new(), |mut body, chunk| async move {
            body.extend_from_slice(&chunk);
            Ok::<_, Error>(body)
        })
        .map_ok(|contents| (filename, contents))
        .and_then(|(filename, contents)| async move {
            // Create the file.
            let f = TokioFile::create(&filename).await;
            if let Err(_) = f {
                println!("Failed to create file \"{}\"", filename);
                return Ok(HttpResponse::NotFound().finish());
            }

            // Write the contents into it.
            if let Err(_) = f.unwrap().write_all(&contents).await {
                println!("Failed to write file \"{}\"", filename);
                return Ok(HttpResponse::NotFound().finish());
            }

            println!("Uploaded file \"{}\"", filename);
            Ok(HttpResponse::Ok().finish())
        })
        .await
}

async fn upload_new_file(payload: web::Payload, info: web::Path<(String,)>) -> impl Responder {
    let filename_prefix = info.0.clone();
    print!("Uploading file \"{}*.txt\" ... ", filename_prefix);
    flush_stdout();

    payload
        .map_err(Error::from)
        .try_fold(web::BytesMut::new(), |mut body, chunk| async move {
            body.extend_from_slice(&chunk);
            Ok::<_, Error>(body)
        })
        .map_ok(move |contents| (filename_prefix, contents))
        .and_then(|(filename_prefix, contents)| async move {
            let mut rng = rand::thread_rng();
            let mut attempts = 0;
            let mut file;
            let mut filename;
            const MAX_ATTEMPTS: u32 = 100;

            loop {
                attempts += 1;
                if attempts > MAX_ATTEMPTS {
                    println!(
                        "Failed to create new file with prefix \"{}\", \
                         after {} attempts.",
                        filename_prefix, MAX_ATTEMPTS
                    );
                    return Ok(HttpResponse::NotFound().finish());
                }

                // Generate a 3-digit pseudo-random number.
                // and use it to create a file name.
                filename = format!("{}{:03}.txt", filename_prefix, rng.gen_range(0, 1000));

                // Create a not-yet-existing file.
                file = TokioFile::create(&filename).await;

                // If it was created, exit the loop.
                if file.is_ok() {
                    break;
                }
            }

            // Write the contents into it asynchronously.
            if file.unwrap().write_all(&contents).await.is_err() {
                println!("Failed to write file \"{}\"", filename);
                return Ok(HttpResponse::NotFound().finish());
            }

            println!("Uploaded file \"{}\"", filename);
            Ok(HttpResponse::Ok().content_type("text/plain").body(filename))
        })
        .await
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

