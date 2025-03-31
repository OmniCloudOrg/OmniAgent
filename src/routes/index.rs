use rocket::get;
use rocket::response::content;
use serde::{Deserialize, Serialize};
use env_logger::{Builder, Target};
use lazy_static::lazy_static;
use reqwest::Client;
use rocket::serde::json::Json;
use rocket::Build;
use rocket::Rocket;
use std::{env, sync::Arc};
use tokio::sync::RwLock;
use std::sync::Mutex;

/// Route information structure for API documentation
#[derive(Serialize, Clone)]
pub struct RouteInfo {
    /// Path of the route
    path: String,
    /// HTTP methods supported by the route
    methods: Vec<String>,
}

/// Response structure for routes listing endpoint
#[derive(Serialize)]
pub struct RoutesResponse {
    /// List of all available routes and their methods
    routes: Vec<RouteInfo>,
}

/// Routes collection that will be populated during startup
#[derive(Clone)]
pub struct RoutesCollection {
    routes: Vec<RouteInfo>,
}

impl RoutesCollection {
    pub fn new() -> Self {
        Self { routes: Vec::new() }
    }

    pub fn add_route(&mut self, path: String, method: String) {
        // Check if the route already exists
        if let Some(route) = self.routes.iter_mut().find(|r| r.path == path) {
            // Add method if it doesn't exist
            if !route.methods.contains(&method) {
                route.methods.push(method);
            }
        } else {
            // Add new route info
            self.routes.push(RouteInfo {
                path,
                methods: vec![method],
            });
        }
    }

    pub fn get_routes(&self) -> Vec<RouteInfo> {
        self.routes.clone()
    }
}

/// Global singleton instance of the routes collection
/// Stores information about all registered API routes
lazy_static! {
    static ref ROUTES_COLLECTION: Arc<Mutex<RoutesCollection>> = Arc::new(Mutex::new(RoutesCollection::new()));
}

/// Decodes any encoded characters in route paths and preserves parameter notation
fn decode_route_path(path: &str) -> String {
    let mut result = path.to_string();
    
    // Handle common Unicode escape sequences
    let replacements = [
        ("\\u003C", "<"), ("\\u003E", ">"),
        ("\\u003c", "<"), ("\\u003e", ">"),
        ("\\u0026", "&"), ("\\u0027", "'"),
        ("\\u0022", "\""), ("\\u003D", "="),
        ("\\u003F", "?"), ("\\u002F", "/"),
    ];
    
    for (encoded, decoded) in replacements.iter() {
        result = result.replace(encoded, decoded);
    }
    
    result
}

fn escape_html_in_path(path: &str) -> String {
    path.replace("<", "&lt;").replace(">", "&gt;")
}

// Keep your existing collect_routes function
pub fn collect_routes(rocket: &Rocket<Build>) {
    let mut routes_collection = ROUTES_COLLECTION.lock().unwrap();
    
    for route in rocket.routes() {
        // Get the path and decode any escaped characters
        let path = decode_route_path(&route.uri.to_string());
        
        routes_collection.add_route(
            path,
            route.method.to_string(),
        );
    }
}

/// Routes listing endpoint providing HTML representation of routes
#[get("/")]
pub fn index() -> content::RawHtml<String> {
    let routes_collection = ROUTES_COLLECTION.lock().unwrap();
    let routes = routes_collection.get_routes();

    // Collect unique versions dynamically
    let mut versions: Vec<String> = routes
        .iter()
        .filter_map(|route| {
            let path = &route.path;
            if let Some(start) = path.find("/api/v") {
                let rest = &path[start + 6..];
                let end = rest.find('/').unwrap_or(rest.len());
                let version = &rest[..end];
                if version.chars().all(|c| c.is_numeric()) {
                    Some(format!("v{}", version))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    versions.sort();
    versions.dedup();

    // Add "All Versions" and "Unversioned" options
    let mut version_options = String::from(
        r#"
        <option value="all">All Versions</option>
        <option value="unversioned">Unversioned</option>
    "#,
    );

    // Add detected versions dynamically
    version_options.push_str(
        &versions
            .iter()
            .map(|v| format!(r#"<option value="{}">{}</option>"#, v.to_lowercase(), v))
            .collect::<String>(),
    );

    // Start building the HTML for the table
    let mut route_rows = String::new();

    // Sort routes for better presentation
    let mut sorted_routes = routes.clone();
    sorted_routes.sort_by(|a, b| a.path.cmp(&b.path));

    // Create table rows
    for route in sorted_routes {
        // Sort methods for consistency
        let mut methods = route.methods.clone();
        methods.sort();

        for method in methods {
            let method_class = method.to_lowercase();
            let escaped_path = escape_html_in_path(&route.path);
            let version = if let Some(start) = route.path.find("/api/v") {
                let rest = &route.path[start + 6..];
                let end = rest.find('/').unwrap_or(rest.len());
                format!("v{}", &rest[..end])
            } else {
                "unversioned".to_string()
            };

            route_rows.push_str(&format!(
                "<tr class=\"route-row\" data-method=\"{}\" data-path=\"{}\" data-version=\"{}\">
                    <td class=\"method-col\"><span class=\"method {}\">{}</span></td>
                    <td class=\"path-col\"><a href=\"{}\" style=\"color: white; text-decoration: none;\">{}</a></td>
                </tr>\n",
                method.to_lowercase(),
                escaped_path.to_lowercase(),
                version.to_lowercase(),
                method_class,
                method,
                escaped_path,
                escaped_path
            ));
        }
    }

    // Complete HTML with search, method, and version filters
    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>OmniAgent API</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
            line-height: 1.6;
            color: #333;
            max-width: 900px;
            margin: 0 auto;
            padding: 20px;
        }}
        h1 {{
            color: #2c3e50;
            border-bottom: 2px solid #3498db;
            padding-bottom: 10px;
        }}
        .container {{
            background-color: #fff;
            border-radius: 8px;
            padding: 25px;
            box-shadow: 0 4px 6px rgba(0,0,0,0.1);
            margin-bottom: 20px;
        }}
        .routes-section {{
            background-color: #fff;
            border-radius: 8px;
            padding: 20px;
            box-shadow: 0 4px 6px rgba(0,0,0,0.1);
        }}
        table {{
            width: 100%;
            border-collapse: separate;
            border-spacing: 0;
        }}
        th, td {{
            text-align: left;
            padding: 12px 15px;
            border-bottom: 1px solid #eee;
        }}
        th {{
            background-color: #f8f9fa;
            font-weight: 600;
        }}
        .method {{
            display: inline-block;
            padding: 6px 10px;
            border-radius: 4px;
            color: white;
            font-weight: bold;
            min-width: 60px;
            text-align: center;
        }}
        .method-col {{
            width: 100px;
        }}
        .get {{
            background-color: #61affe;
        }}
        .post {{
            background-color: #49cc90;
        }}
        .put {{
            background-color: #fca130;
        }}
        .delete {{
            background-color: #f93e3e;
        }}
        .patch {{
            background-color: #9c42be;
        }}
        /* Search and Filter Styles */
        .search-container {{
            display: flex;
            gap: 10px;
            margin-bottom: 15px;
        }}
        #searchInput, #methodFilter, #versionFilter {{
            padding: 10px;
            width: calc(33.33% - 10px);
            border: 1px solid #ccc;
            border-radius: 8px;
            background-color: #f8f9fa;
            color: #333;
            font-size: 14px;
            transition: all 0.3s ease;
        }}
        #searchInput:focus, #methodFilter:focus, #versionFilter:focus {{
            outline: none;
            border-color: #3498db;
            box-shadow: 0 0 5px rgba(52, 152, 219, 0.5);
        }}
        option {{
            padding: 10px;
        }}
        /* Dark Mode Styles */
        @media (prefers-color-scheme: dark) {{
            body {{
                background-color: #1a1a1a;
                color: #e0e0e0;
            }}
            .container, .routes-section {{
                background-color: #2d2d2d;
                box-shadow: 0 4px 6px rgba(0,0,0,0.3);
            }}
            th {{
                background-color: #3d3d3d;
            }}
            td {{
                border-bottom: 1px solid #444;
            }}
            h1 {{
                color: #81a1c1;
                border-bottom-color: #5e81ac;
            }}
            #searchInput, #methodFilter, #versionFilter {{
                background-color: #2d2d2d;
                color: #e0e0e0;
                border: 1px solid #444;
            }}
            #searchInput:focus, #methodFilter:focus, #versionFilter:focus {{
                border-color: #81a1c1;
                box-shadow: 0 0 5px rgba(94, 129, 172, 0.5);
            }}
        }}
    </style>
</head>
<body>
    <div class="container">
        <h1>Welcome to OmniAgent</h1>
        <p>OmniAgent is a distributed system for managing application instances within the OmniCloud platform. Please refer to the API documentation below to get started!</p>
    </div>

    <div class="routes-section">
        <h2>Available Routes</h2>

        <!-- Search Bar and Filters -->
        <div class="search-container">
            <input type="text" id="searchInput" placeholder="Search routes by path..." onkeyup="filterRoutes()">
            <select id="methodFilter" onchange="filterRoutes()">
                <option value="all">All Methods</option>
                <option value="get">GET</option>
                <option value="post">POST</option>
                <option value="put">PUT</option>
                <option value="delete">DELETE</option>
                <option value="patch">PATCH</option>
            </select>
            <select id="versionFilter" onchange="filterRoutes()">
                {version_options}
            </select>
        </div>

        <table>
            <thead>
                <tr>
                    <th>Method</th>
                    <th>Path</th>
                </tr>
            </thead>
            <tbody id="routesTable">
                {route_rows}
            </tbody>
        </table>
    </div>

    <script>
        function filterRoutes() {{
            let input = document.getElementById('searchInput').value.toLowerCase();
            let methodFilter = document.getElementById('methodFilter').value.toLowerCase();
            let versionFilter = document.getElementById('versionFilter').value.toLowerCase();
            let rows = document.querySelectorAll('.route-row');

            rows.forEach(row => {{
                let method = row.getAttribute('data-method').toLowerCase();
                let path = row.getAttribute('data-path').toLowerCase();
                let version = row.getAttribute('data-version').toLowerCase();
                
                // Match method, version, and path with filters
                let methodMatch = methodFilter === 'all' || method === methodFilter;
                let versionMatch = 
                    versionFilter === 'all' || 
                    (versionFilter === 'unversioned' && version === 'unversioned') || 
                    version === versionFilter;
                let pathMatch = path.includes(input);

                // Show row if all conditions match
                if (methodMatch && versionMatch && pathMatch) {{
                    row.style.display = '';
                }} else {{
                    row.style.display = 'none';
                }}
            }});
        }}
    </script>
</body>
</html>"#
    );

    content::RawHtml(html)
}
