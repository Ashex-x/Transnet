# app.py

import flask
import flask_cors
import typing

import backend.utils

import logging

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

web_application = flask.Flask(__name__)
flask_cors.CORS(web_application)


@web_application.route('/')
def serve_index():
  """Home page"""
  return flask.send_from_directory(
    'static',
    'index_temp.html'
    )


@web_application.route('/<path:filename>')
def serve_static_files(filename):
    """Other static files"""
    return flask.send_from_directory('static', filename)


@web_application.route('/api/input-text', methods=['POST'])
def translate_input_text():
  """
  Receives input data that needs to be translate from frontend.
  Sends translation result to frontend.
  """
  try:
    data = flask.request.get_json()
    if data is None:
      return flask.jsonify({"error": "No JSON patload received, \
                            but request was sent"}), 400

    if not isinstance(data, typing.Dict):
      raise TypeError("Expected a dictionary, got {}".
                      format(type(data).__name__))

    # Receives input data
    input_timestamp = data.get('timestamp')
    input_text = data.get('text')
    # input_source = data.get('source')
    # input_length = data.get('input_length')

    output_text = None
    if input_text is not None:
      output_text = input_text  # Translates the input text.
      return flask.jsonify({
        "status": "success",
        "message": "Data received successfully.",
        "output": output_text,
        "char_count": len(output_text),
        "processed_timestamp": input_timestamp
      }), 200
    else:
      return flask.jsonify({"error":
                            "Input is empty by somehow(unknown)."}), 500

  except Exception as e:
    return flask.jsonify({"error": str(e)}), 500


def run_frontend():
  web_application.run(host="0.0.0.0", port=13579, debug=True)
  # back_utils = backend.utils.PyUtils()
