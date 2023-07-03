use std::fmt::Write;

use actix_web::http::header::ContentType;
use actix_web::HttpResponse;
use actix_web_flash_messages::IncomingFlashMessages;

pub async fn publish_newsletter_form(
    flash_messages: IncomingFlashMessages,
) -> Result<HttpResponse, actix_web::Error> {
    let mut msg_html = String::new();
    for m in flash_messages.iter() {
        writeln!(msg_html, "<p><i>{}</i></p>", m.content()).unwrap();
    }

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"<!DOCTYPE html>
<html lang="en>
<head>
    <meta http-equiv="content-type" content="text/html; charset=utf-8">
    <title>Send Newsletter</title>
</head>
<body>
    {msg_html}
    <p>Submit a newsletter</p>
    <form name="submitNewsletter" action="/admin/newsletter" method="post">
        <label>Title<br>
            <input
                type="text"
                placeholder="Newsletter Title"
                name="title"
            >
        </label
        <br>
        <label>HTML<br>
            <textarea
                placeholder="Enter the plain text of newsletter"
                name="html_content"
                rows="20"
                cols="50"
            ></textarea>
        </label
        <br>
        <label>Plain Text<br>
            <textarea
                    placeholder="Enter the plain text of newsletter"
                    name="text_content"
                    rows="20"
                    cols="50"
                ></textarea>
        </label
        <br>
        <button type="submit">Publish</button>
    </form>
    <p><a href="/admin/dashboard">&lt;- Back</a><p>
</body>
</html>
    "#,
        )))
}
