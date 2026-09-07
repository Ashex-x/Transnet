# Account API

Parent: [Gateway HTTP API](README.md).

## Contents

- [Overview](#overview)
- [Authentication](#authentication)
- [Interfaces](#interfaces)
- [Routes](#routes)

## Overview

Account routes preserve the existing client-facing shape but do not validate credentials, persist users, rotate passwords, or revoke tokens.

## Authentication

Account routes require no credentials. Returned tokens are mock values and are not valid authentication.

## Interfaces

Responses use the [gateway envelopes and shared wire rules](README.md).

## Routes

## POST /api/account/register

Accepts `username`, `email`, and `password`, then returns a mock user with status 201. No account is persisted.

## POST /api/account/login

Accepts `email` and `password`, then returns mock access and refresh tokens. Credentials are not validated.

## POST /api/account/logout

Returns a static success message and does not revoke tokens.

## POST /api/account/refresh

Accepts `refresh_token` and returns a mock access token without validation.

## POST /api/account/change-password

Returns a static success message and does not change credentials.
