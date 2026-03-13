# Transnet Gateway Server

HTTP gateway server that serves frontend and forwards translation requests to backend Transnet server.

## Architecture

Frontend (TS/HTML) → transnet-server (this gateway) → HTTP → island-transnet/bins/transnet-server (backend core server)

## Setup

1. Build the frontend:
  ```bash
  cd static
  npm install
  npm run build
  cd ..
  ```

2. Configure environment variables in the root `.env` file:
  - `GATEWAY_PORT` - Port for this gateway server (default: 35791)
  - `BACKEND_HOST` - Backend server host (default: 127.0.0.1)
  - `BACKEND_PORT` - Backend server port (default: 35792)

3. Build and run:
  ```bash
  cd transnet-server
  cargo run
  ```

4. Access the frontend at:
  ```
  http://localhost:35791
  ```

## API Endpoints

### Static File Serving
- `/` - Serves the frontend application (index.html)
- `/assets/*` - Serves static assets from `static/dist/`
- `/resource/*` - Serves resources from `static/resource/`

### `GET /health`
Check server health status.

### `POST /translate`
Submit translation request.

Request body:
```json
{
  "text": "Hello world",
  "source_lang": "en",
  "target_lang": "es",
  "mode": null,
  "input_type": null
}
```

## Configuration

Environment variables (configured in root `.env`):
- `GATEWAY_HOST` - Host to bind to (default: 127.0.0.1)
- `GATEWAY_PORT` - Port to listen on (default: 35791)
- `BACKEND_HOST` - Backend server host (default: 127.0.0.1)
- `BACKEND_PORT` - Backend server port (default: 35792)
- `WORKERS` - Number of worker threads (default: 4)
- `RUST_LOG` - Logging level (default: info)

## Development

For development, use watch mode for the frontend:

```bash
# In one terminal, watch the frontend
cd static
npm run watch

# In another terminal, run the server
cd transnet-server
cargo run
```

## Frontend Build

See [static/README.md](../static/README.md) for detailed frontend build instructions.
