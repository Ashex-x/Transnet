# app.py

import os
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
def receive_input_text():
  """s
  Receives input data that needs to be translate from frontend.
  """
  try:
    data = flask.request.get_json()
    if data is None:
      return flask.jsonify({"error": "No JSON patload received"}), 400

    if not isinstance(data, typing.Dict):
      raise TypeError("Expected a dictionary, got {}".
                      format(type(data).__name__))

    input_timestamp = data.get('timestamp')
    # input_text = data.get('text')
    # input_source = data.get('source')
    # input_length = data.get('input_length')

    return flask.jsonify({
      "status": "success",
      "message": "Data received successfully.",
      "processed_timestamp": input_timestamp
    }), 200

  except Exception as e:
    return flask.jsonify({"error": str(e)}), 500


if __name__ == "__main__":
  web_application.run(host="0.0.0.0", port=13579, debug=True)
  # back_utils = backend.utils.PyUtils()
