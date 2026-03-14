# Database

This document describes the databases used in Transnet.

**Dataflow**: Client → Web server → Backend server

- **Web server**: MySQL — user accounts, profiles, and per-user data (history, favorites).
- **Backend server**: RAG database — used for translation and retrieval; documented separately later.

---

## Web server (MySQL)

The web server uses MySQL to store user identity, profiles, and the data tied to each user.

### Users

Stores user accounts and profile information.

| Field         | Type      | Notes                          |
| ------------- | --------- | ------------------------------ |
| user_id       | uuid      | Primary key (e.g. UUID v7)     |
| username      | string    | 3–50 chars, unique             |
| email         | string    | Valid email, unique            |
| password_hash | string    | Hashed (e.g. bcrypt)           |
| updated_at    | timestamp | Last profile update            |
| active        | boolean   | Optional; for account inactive |

### History

Per-user translation history. Each user has a list of past translations. This is the data behind the `/history` API. **Favorites** are the same records with `is_favorite = true`; get favorites by filtering the user’s history on this flag. The `note` field stores an optional note for favorited items (empty by default).

| Field         | Type      | Notes                             |
| ------------- | --------- | --------------------------------- |
| translation_id| uuid      | Primary key                       |
| user_id       | uuid      | FK → Users                        |
| text          | string    | Source text                       |
| translation   | string    | Translated text                   |
| source_lang   | string    | Language code (e.g. en)           |
| target_lang   | string    | Language code                     |
| input_type    | string    | e.g. word, phrase, sentence, text |
| provider      | string    | e.g. openai-compatible            |
| model         | string    | Model name                        |
| created_at    | timestamp | When the translation was made     |
| is_favorite   | boolean   | Default false; true = in favorites|
| note          | string?   | Optional, max 500 chars; default empty (used when favorited) |
| updated_at    | timestamp | Optional; last note update        |
