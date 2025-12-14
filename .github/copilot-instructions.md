# Transnet AI Coding Instructions

## Architecture Overview
Transnet implements a multi-system translation platform with Flask backend and static frontend:

### Core Systems
- **Basic Translation System (BTS)**: Dictionary-based translation using CEDict/CCDict CSV files loaded into pandas DataFrame
- **Related Search System (RSeS)**: LLM-powered semantic relation discovery using Deepseek-v3 API for hypernyms/hyponyms/synonyms
- **Related Storage System (RStS)**: SQLite-based knowledge base for storing user feedback-weighted relations and extensions

### Data Flow
Frontend (static/index_temp.html + js/main_temp.js) → Flask API (/api/input-text) → backend/utils.PyUtils.total_trans() → BTS lookup → JSON response with traditional/simplified/pinyin/translation fields

### Planned Integrations
- **G2_trans (Gemma2)**: For unknown words and essay translation
- **G2_relate**: Core knowledge extraction for detailed word analysis
- **Gemini-2.5/3 API**: Academic paper translation
- **Personal History Database (PHD)**: SQLite for user-specific translation history
- **Trie Structure**: In-memory dictionary core for performance

## Key Workflows
- **Development Server**: `python runme.py` starts Flask on 0.0.0.0:13579 with debug=True
- **Testing**: `python runme_test.py` runs backend.utils_test.main() for basic translation validation
- **Data Processing**: Convert raw dictionaries in database/raw_data/ to CSV format for BTS consumption
- **C++ Extensions**: CMake build in backend/cpp/ for performance-critical operations (currently stubbed)

## Project Conventions
- **Translation Response Format**: Always return `{"tranditional": str, "simplified": str, "pinyin": str, "translation": list}`
- **Dictionary Priority**: Core dict → Source dicts → Model fallback (G2_trans for unknowns)
- **Logging**: Mandatory Python logging with `logging.getLogger()`; logs written to logs/ directory
- **File Paths**: Relative paths using `os.path.join(current_dir, "../../database/processed_data/cedict.csv")`
- **Frontend Development**: Use index_temp.html and main_temp.js for backend integration work
- **Database Choice**: pandas for small dicts, SQLite for large PHD/relation storage

## Integration Points
- **LLM APIs**: Deepseek-v3 for relations, Gemini for essays (API key management needed)
- **Feedback System**: Upvote/downvote mechanism for relation weighting in RStS
- **Offline Capability**: Planned MarianMT deployment for local translation
- **Multi-format Support**: Word/phrase/sentence/essay translation with automatic model switching

## Examples
- **BTS Lookup**: `basic_trans.BasicTrans.trans("你好")` queries DataFrame for simplified match
- **API Error Response**: `flask.jsonify({"error": "No JSON payload"}), 400`
- **Relation Storage**: SQLite tables for extension database and weighted relation graph
- **Model Switching**: If no dict result, route to G2_trans for unknown words</content>
<parameter name="filePath">/home/ashex/Works/Transnet/.github/copilot-instructions.md