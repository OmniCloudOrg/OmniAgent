use rocket::{get, State};
use rocket::response::content;
use rocket::http::Method;
use rocket::Route;

// Function to generate HTML representation of routes
fn generate_routes_html(routes: &[Route]) -> String {
    let mut routes_html = String::from(r#"<div class="routes-section">
        <h2>Available Routes</h2>
        <table class="routes-table">
            <thead>
                <tr>
                    <th>Method</th>
                    <th>Path</th>
                    <th>Format</th>
                </tr>
            </thead>
            <tbody>
    "#);

    for route in routes {
        let method = match route.method {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Delete => "DELETE",
            Method::Options => "OPTIONS",
            Method::Head => "HEAD",
            Method::Patch => "PATCH",
            _ => "OTHER"
        };

        routes_html.push_str(&format!(r#"
            <tr>
                <td class="method method-{}">{}</td>
                <td>{}</td>
                <td>{}</td>
            </tr>
        "#, 
            method.to_lowercase(),
            method,
            route.uri.path(),
            route.format.as_ref().map_or("any".to_string(), |f| f.to_string())
        ));
    }

    routes_html.push_str(r#"
            </tbody>
        </table>
    </div>"#);

    routes_html
}

#[get("/")]
pub fn index(routes: &State<Vec<Route>>) -> content::RawHtml<String> {
    let routes_html = generate_routes_html(routes);
    
    content::RawHtml(format!(r#"
    <!DOCTYPE html>
    <html>
    <head>
        <title>OmniAgent</title>
        <style>
            :root {{
                --bg-color: #f8f9fa;
                --container-bg: #ffffff;
                --text-color: #333333;
                --heading-color: #2c3e50;
                --border-color: #3498db;
                --secondary-text: #555555;
                --shadow-color: rgba(0,0,0,0.1);
                --get-color: #61affe;
                --post-color: #49cc90;
                --put-color: #fca130;
                --delete-color: #f93e3e;
            }}
            
            @media (prefers-color-scheme: dark) {{
                :root {{
                    --bg-color: #121212;
                    --container-bg: #1e1e1e;
                    --text-color: #e0e0e0;
                    --heading-color: #81a1c1;
                    --border-color: #5e81ac;
                    --secondary-text: #c0c0c0;
                    --shadow-color: rgba(0,0,0,0.3);
                }}
            }}
            
            body {{
                font-family: 'Inter', system-ui, -apple-system, sans-serif;
                line-height: 1.7;
                max-width: 800px;
                margin: 0 auto;
                padding: 2.5rem;
                color: var(--text-color);
                background-color: var(--bg-color);
                transition: background-color 0.3s, color 0.3s;
            }}
            
            h1, h2 {{
                color: var(--heading-color);
                margin-top: 0;
                font-weight: 600;
            }}
            
            h1 {{
                border-bottom: 2px solid var(--border-color);
                padding-bottom: 0.7rem;
            }}
            
            p {{
                font-size: 1.1rem;
                color: var(--secondary-text);
                margin-bottom: 1.5rem;
            }}
            
            .container {{
                background-color: var(--container-bg);
                border-radius: 12px;
                padding: 2.5rem;
                box-shadow: 0 4px 20px var(--shadow-color);
                margin-bottom: 1.5rem;
            }}
            
            .routes-section {{
                background-color: var(--container-bg);
                border-radius: 12px;
                padding: 2rem;
                box-shadow: 0 4px 20px var(--shadow-color);
            }}
            
            .routes-table {{
                width: 100%;
                border-collapse: collapse;
                margin-top: 1rem;
            }}
            
            .routes-table th, .routes-table td {{
                padding: 0.75rem;
                text-align: left;
                border-bottom: 1px solid var(--border-color);
            }}
            
            .routes-table th {{
                font-weight: 600;
            }}
            
            .method {{
                font-weight: bold;
                padding: 0.25rem 0.5rem;
                border-radius: 4px;
                display: inline-block;
                text-align: center;
            }}
            
            .method-get {{ background-color: var(--get-color); color: white; }}
            .method-post {{ background-color: var(--post-color); color: white; }}
            .method-put {{ background-color: var(--put-color); color: white; }}
            .method-delete {{ background-color: var(--delete-color); color: white; }}
        </style>
    </head>
    <body>
        <div class="container">
            <h1>Welcome to OmniAgent</h1>
            <p>OmniAgent is a lightweight agent for managing Docker containers. Please refer to the API documentation for the agent to get started!</p>
        </div>
        
        {routes_html}
    </body>
    </html>
    "#))
}