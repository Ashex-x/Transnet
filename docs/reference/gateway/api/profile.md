# Profile API

Parent: [Gateway HTTP API](README.md).

## Contents

- [Overview](#overview)
- [Authentication](#authentication)
- [Interfaces](#interfaces)
- [Routes](#routes)

## Overview

Profile routes reserve the existing paths while profile storage and authentication are not implemented.

## Authentication

Both routes require authentication, but the gateway does not yet validate credentials. They therefore always return `401 UNAUTHORIZED`.

## Interfaces

Responses use the [gateway envelopes and shared wire rules](README.md).

## Routes

## GET /api/profile

Returns `401 UNAUTHORIZED` with the message `Authentication required`.

## PUT /api/profile

Returns `401 UNAUTHORIZED` with the message `Authentication required`; request content is not processed.
