use rocket::get;
use rocket::response::content;

#[get("/")]
pub async fn index() -> content::RawHtml<&'static str> {
    content::RawHtml(r#"
    <!DOCTYPE html>
    <html>
    <head>
        <title>OmniAgent</title>
        <style>
            :root {
                --bg-color: #f8f9fa;
                --container-bg: #ffffff;
                --text-color: #333333;
                --heading-color: #2c3e50;
                --border-color: #3498db;
                --secondary-text: #555555;
                --shadow-color: rgba(0,0,0,0.1);
            }
            
            @media (prefers-color-scheme: dark) {
                :root {
                    --bg-color: #121212;
                    --container-bg: #1e1e1e;
                    --text-color: #e0e0e0;
                    --heading-color: #81a1c1;
                    --border-color: #5e81ac;
                    --secondary-text: #c0c0c0;
                    --shadow-color: rgba(0,0,0,0.3);
                }
            }
            
            body {
                font-family: 'Inter', system-ui, -apple-system, sans-serif;
                line-height: 1.7;
                max-width: 800px;
                margin: 0 auto;
                padding: 2.5rem;
                color: var(--text-color);
                background-color: var(--bg-color);
                transition: background-color 0.3s, color 0.3s;
            }
            
            h1 {
                color: var(--heading-color);
                border-bottom: 2px solid var(--border-color);
                padding-bottom: 0.7rem;
                margin-top: 0;
                font-weight: 600;
            }
            
            p {
                font-size: 1.1rem;
                color: var(--secondary-text);
                margin-bottom: 0;
            }
            
            .container {
                background-color: var(--container-bg);
                border-radius: 12px;
                padding: 2.5rem;
                box-shadow: 0 4px 20px var(--shadow-color);
                transition: background-color 0.3s, box-shadow 0.3s;
            }
        </style>
    </head>
    <body>
        <div class="container">
            <h1>Welcome to OmniAgent</h1>
            <p>OmniAgent is a lightweight agent for managing Docker containers. Please refer to the API documentation for the agent to get started!</p>
        </div>
    </body>
    </html>
    "#)
}