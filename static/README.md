# Transnet Frontend

Frontend application for Transnet translation service.

## Project Structure

```
static/
├── src/              # TypeScript source code
│   ├── components/   # Web components
│   ├── flatbuffers/ # FlatBuffers types
│   ├── styles/      # SCSS source files
│   ├── app.ts       # Main application entry point
│   └── router.ts    # Router configuration
├── resource/        # Static resources (favicon, images, etc.)
├── index.html       # Main HTML entry point
├── package.json     # NPM dependencies and scripts
├── tsconfig.json    # TypeScript configuration
└── fix-imports.js  # Post-build import fixer
```

## Prerequisites

- Node.js (v18 or higher)
- npm

## Installation

```bash
cd static
npm install
```

## Development

### Build frontend

```bash
# Full build (cleans, compiles TypeScript, fixes imports, builds SCSS)
npm run build

# Development build (without clean)
npm run dev
```

### Watch mode for development

```bash
# Watch both TypeScript and SCSS files
npm run watch

# Or watch separately
npm run watch:ts    # Watch TypeScript only
npm run watch:scss  # Watch SCSS only
```

### Clean build artifacts

```bash
npm run clean
```

## Build Process

1. **TypeScript compilation**: `tsc` compiles TypeScript to JavaScript in `dist/`
2. **Import fixing**: `node fix-imports.js` fixes ES module imports to include `.js` extensions
3. **SCSS compilation**: `sass` compiles SCSS to CSS in `dist/styles/`

## Deployment

The frontend is served by Rust backend at `transnet-server/`.

### Before running the server

Make sure to build the frontend:

```bash
cd static
npm install
npm run build
cd ..
```

### Access the application

Once the server is running, access the frontend at:
```
http://localhost:35791
```

## Static Assets

- `/assets/*` - Serves compiled frontend assets (JS, CSS)
- `/resource/*` - Serves static resources (favicon, images)

## Notes

- The `dist/` directory is generated and should not be committed to version control
- The `node_modules/` directory should not be committed to version control
- FlatBuffers is loaded from CDN (jsDelivr) for better performance
- Always run `npm install` after cloning the repository
