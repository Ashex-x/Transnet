# API design
In this document, we will design the API for the Transnet project.

The basic workflow is: Client(TS) -> Web Server(PHP) -> Backend Core(Rust)

## Frontend API (Client TS)
We wrote frontend in TypeScript.

### Account Management
`POST /account/register`: Register a new account.  
Request Body:
```json
{
  "email": "test@example.com",
  "password": "password"
}
```

Response Body:
```json
{
  "success": true,
  "message": "Account registered successfully"
}
```
`POST /account/login`:
Login to the account.

`POST /account/logout`:
Logout from the account.

`POST /api/v1/account/forgot-password`:
Forgot password.

`POST /api/v1/account/reset-password`:
Reset password.

`POST /api/v1/account/verify-email`:
Verify email.

`POST /api/v1/account/resend-verification-email`:
Resend verification email.

`POST /api/v1/account/send-password-reset-email`
Send password reset email.

### Translation
`POST /api/v1/translation/translate`:
Translate text.

### History
`GET /api/v1/history`:
Get history.


## Backend API (Server PHP)


## 