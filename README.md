This project is a simple HTTP API server written in Rust using Hyper and Reqwest.
It accepts POST requests with JSON data at /hello, validates the content type, and forwards the JSON payload to an external API (https://postman-echo.com/post).
The response from the external API is returned to the client.
