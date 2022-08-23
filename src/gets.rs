use worker::*;

use crate::{error, BUCKET};

pub async fn template(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    if let Ok(bkt) = ctx.bucket(BUCKET) {
        let key = match ctx.param("id") {
            Some(val) => val,
            None => return error("No Image ID!", 400),
        };
        let files = match bkt.list().prefix(key).execute().await {
            Ok(val) => val.objects(),
            Err(e) => return error(&format!("R2 Error: {}", e), 500),
        };

        if files.is_empty() {
            return error("Imageset not found!", 404);
        }

        let mut context = tera::Context::new();
        context.insert("id", key);
        context.insert(
            "images",
            &files
                .iter()
                .map(|f| {
                    urlencoding::decode(
                        f.key()
                            .split_once('/')
                            .unwrap_or(("", "(unable to split filename)"))
                            .1,
                    )
                    .unwrap_or(std::borrow::Cow::Borrowed("(unable to decode filename)"))
                    .into_owned()
                })
                .collect::<Vec<String>>(),
        );
        if let Ok(page) = tera::Tera::one_off(include_str!("html/img.html"), &context, true) {
            return Response::from_html(page);
        }

        return error(
            "Templating failed! (this is a bug, github.com/randomairborne/imgify)",
            500,
        );
    }
    Response::error("Account Misconfigured, no imgify kv found", 500)
}

pub async fn raw(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    if let Ok(bkt) = ctx.bucket(BUCKET) {
        let id = match ctx.param("id") {
            Some(val) => val,
            None => return Response::error("No image ID!", 400),
        };
        let name = match ctx.param("name") {
            Some(val) => val,
            None => return Response::error("No image name!", 400),
        };
        let maybe_value = match bkt.get(format!("{id}/{name}")).execute().await {
            Ok(val) => val,
            Err(e) => return Response::error(&format!("R2 Error: {}", e), 500),
        };
        if let Some(value) = maybe_value {
            let body = if let Some(body) = value.body() {
                body
            } else {
                return Response::error("Failed to get image body", 500);
            };
            let body_stream = match body.stream() {
                Ok(stream) => stream,
                Err(e) => {
                    return Response::error(&format!("Failed to get body as stream: {e}"), 500)
                }
            };
            return Response::from_stream(body_stream);
        }
        return Response::error("Image Not Found!", 404);
    }
    Response::error("Account Misconfigured, no imgify kv found", 500)
}

pub mod style {
    use worker::*;
    pub fn main(_req: Request, _ctx: RouteContext<()>) -> Result<Response> {
        let mut headers = Headers::new();
        headers.append("Content-Type", "text/css")?;
        Ok(Response::ok(include_str!("html/main.css"))?.with_headers(headers))
    }
    pub fn submit(_req: Request, _ctx: RouteContext<()>) -> Result<Response> {
        let mut headers = Headers::new();
        headers.append("Content-Type", "text/css")?;
        Ok(Response::ok(include_str!("html/index.css"))?.with_headers(headers))
    }
}
