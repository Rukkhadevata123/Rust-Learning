use actix_files as fs;
use actix_web::{web, App, HttpResponse, HttpServer};
use serde::Deserialize;

// GCD macro definition
macro_rules! gcd {
    ($a:expr, $b:expr) => {{
        let mut a = $a;
        let mut b = $b;
        while b != 0 {
            let t = b;
            b = a % b;
            a = t;
        }
        a
    }};
}

#[derive(Deserialize)]
struct GcdParameters {
    a: u64,
    b: u64,
}

async fn post_gcd(form: web::Form<GcdParameters>) -> HttpResponse {
    if form.a == 0 || form.b == 0 {
        let error_response = r#"
            <html>
            <head><title>Error</title><link rel="stylesheet" href="/static/style.css"></head>
            <body>
                <div class='container'>
                    <div class='calculator-box'>
                        <h1>Error</h1>
                        <p>Cannot compute GCD for zero values. Please go back and enter valid numbers.</p>
                        <a href="/" class="submit-btn">Back to Calculator</a>
                    </div>
                </div>
            </body>
            </html>
        "#;
        return HttpResponse::BadRequest()
            .content_type("text/html")
            .body(error_response);
    }

    let response = format!(
        r#"
        <html>
        <head><title>GCD Result</title><link rel="stylesheet" href="/static/style.css"></head>
        <body>
            <div class='container'>
                <div class='calculator-box'>
                    <h1>GCD Result</h1>
                    <p class="result">
                        The greatest common divisor of the numbers {} and {} is <b>{}</b>.
                    </p>
                    <a href="/" class="submit-btn">Back to Calculator</a>
                </div>
            </div>
        </body>
        </html>
        "#,
        form.a,
        form.b,
        gcd!(form.a, form.b)
    );

    HttpResponse::Ok().content_type("text/html").body(response)
}

#[actix_web::main]
async fn main() {
    let server = HttpServer::new(|| {
        App::new()
            // Serve static files from the "static" folder
            .service(fs::Files::new("/static", "./static").show_files_listing())
            // Serve the GCD form
            .route("/gcd", web::post().to(post_gcd))
            // Serve the index.html as the main page
            .route(
                "/",
                web::get().to(|| async {
                    fs::NamedFile::open_async("./static/index.html")
                        .await
                        .unwrap()
                }),
            )
    });

    println!("Starting server on http://localhost:3000");
    server
        .bind("127.0.0.1:3000")
        .expect("Cannot bind to port 3000")
        .run()
        .await
        .expect("Error running server");
}
