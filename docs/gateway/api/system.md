# System API

## Contents

- [Overview](#overview)
- [Authentication](#authentication)
- [Interfaces](#interfaces)
- [Routes](#routes)

## Overview

System routes expose backend liveness, gateway readiness, product metadata, and compatibility statistics. They do not serve browser assets or provide durable monitoring metrics.

## Authentication

System routes require no credentials.

## Interfaces

Responses use the [gateway envelopes and shared wire rules](../api.md).

## Routes

## GET /health

Proxies backend health. Success returns the backend health object in the standard envelope. Backend failures return `503 BACKEND_ERROR`.

## GET /api/health

Reports gateway readiness only after the backend health request succeeds. Success data is `{"status":"ready","service":"transnet"}`. Backend failures return `503 SERVICE_UNAVAILABLE` without backend details.

## GET /api/about

Returns static service metadata, features, supported languages, and `max_text_length`.

## GET /api/stats

Returns static compatibility counters and dependency status strings. Do not use this stub response for monitoring.
