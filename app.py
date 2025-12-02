from flask import Flask, request, jsonify, send_from_directory
import os
from datetime import datetime
import time

app = Flask(__name__,
            static_folder='static',
            static_url_path='')

# 记录应用启动时间
app_start_time = time.time()


@app.route('/')
def index():
    """主页"""
    return send_from_directory('static', 'index.html')


@app.route('/<path:path>')
def serve_static(path):
    """静态文件服务"""
    return send_from_directory('static', path)


@app.route('/api/process', methods=['POST', 'GET', 'PUT', 'DELETE'])
def process_text():
    """处理文本的API端点"""

    # 记录请求开始时间
    request_start_time = time.time()

    try:
        # 根据请求方法获取数据
        if request.method == 'POST':
            data = request.get_json()
        elif request.method == 'GET':
            data = request.args.to_dict()
        else:
            if request.is_json:
                data = request.get_json()
            elif request.form:
                data = request.form.to_dict()
            else:
                data = {}

        # 验证输入
        if not data or 'text' not in data:
            return jsonify({
                'success': False,
                'error': '缺少文本输入',
                'message': '请提供text字段'
            }), 400

        text = data['text']

        # 简单的处理逻辑示例
        processed_result = {
            'original_text': text,
            'processed_text': text.upper(),  # 转换为大写
            'reversed_text': text[::-1],     # 反转文本
            'length': len(text),
            'word_count': len(text.split()),
            'char_count': {
                'total': len(text),
                'letters': sum(c.isalpha() for c in text),
                'digits': sum(c.isdigit() for c in text),
                'spaces': text.count(' '),
                'others': len(text) - sum(c.isalpha() for c in text) - sum(
                    c.isdigit() for c in text) - text.count(' ')
            },
            'timestamp': datetime.now().isoformat()
        }

        # 模拟处理时间（根据文本长度）
        process_time = min(len(text) * 0.001, 0.5)  # 每字符1ms，最多500ms
        time.sleep(process_time)

        # 计算总响应时间
        total_time = time.time() - request_start_time

        response = {
            'success': True,
            'data': processed_result,
            'message': '文本处理成功',
            'processing_info': {
                'method': request.method,
                'process_time_ms': round(process_time * 1000, 2),
                'total_time_ms': round(total_time * 1000, 2),
                'server_timestamp': datetime.now().isoformat()
            }
        }

        return jsonify(response), 200

    except Exception as e:
        return jsonify({
            'success': False,
            'error': '服务器内部错误',
            'message': str(e),
            'timestamp': datetime.now().isoformat()
        }), 500


@app.route('/api/health', methods=['GET'])
def health_check():
    """健康检查端点"""
    uptime = time.time() - app_start_time

    # 将秒转换为可读格式
    days = int(uptime // (24 * 3600))
    uptime = uptime % (24 * 3600)
    hours = int(uptime // 3600)
    uptime %= 3600
    minutes = int(uptime // 60)
    seconds = int(uptime % 60)

    uptime_str = f"{days}d {hours}h {minutes}m {seconds}s"

    return jsonify({
        'status': 'healthy',
        'service': '文本处理API',
        'version': '1.0.0',
        'server_time': datetime.now().isoformat(),
        'uptime': uptime_str,
        'uptime_seconds': round(time.time() - app_start_time, 2)
    })


@app.route('/api/test', methods=['GET'])
def test_endpoint():
    """测试端点"""
    return jsonify({
        'success': True,
        'message': 'API服务运行正常',
        'timestamp': datetime.now().isoformat(),
        'endpoints': [
            {'path': '/api/process', 'methods': ['POST', 'GET', 'PUT', 'DELETE']
             , 'description': '处理文本'},
            {'path': '/api/health', 'methods': ['GET'], 'description': '健康检查'},
            {'path': '/api/test', 'methods': ['GET'], 'description': '测试接口'}
        ]
    })


@app.route('/api/analyze', methods=['POST'])
def analyze_text():
    """文本分析端点"""
    try:
        data = request.get_json()
        text = data.get('text', '')

        # 计算各种统计信息
        words = text.split()
        sentences = text.replace('!', '.').replace('?', '.').split('.')
        sentences = [s.strip() for s in sentences if s.strip()]

        analysis = {
            'text_length': len(text),
            'word_count': len(words),
            'sentence_count': len(sentences),
            'avg_word_length': round(sum(len(w) for w in words) / max(
                len(words), 1), 2),
            'avg_sentence_length': round(len(words) / max(
                len(sentences), 1), 2),
            'char_frequency': {char: text.count(char) for char in set(text) if
                               char.isalnum()},
            'most_common_words': get_most_common(words),
            'readability_score': calculate_readability(text)
        }

        return jsonify({
            'success': True,
            'data': analysis,
            'message': '文本分析完成'
        })

    except Exception as e:
        return jsonify({
            'success': False,
            'error': '分析失败',
            'message': str(e)
        }), 500


def get_most_common(words, top_n=5):
    """获取最常见的词语"""
    from collections import Counter
    word_counts = Counter(words)
    return dict(word_counts.most_common(top_n))


def calculate_readability(text):
    """计算简单的可读性分数（示例）"""
    words = text.split()
    sentences = text.replace('!', '.').replace('?', '.').split('.')
    sentences = [s.strip() for s in sentences if s.strip()]

    if len(words) == 0 or len(sentences) == 0:
        return 0

    avg_sentence_len = len(words) / len(sentences)
    avg_word_len = sum(len(w) for w in words) / len(words)

    # 简单的可读性公式（示例）
    score = 100 - (avg_sentence_len * 1.5 + avg_word_len * 10)
    return max(0, min(100, round(score, 2)))


# 错误处理
@app.errorhandler(404)
def not_found(error):
    return jsonify({
        'success': False,
        'error': '资源未找到',
        'message': f'请求的路径 {request.path} 不存在',
        'timestamp': datetime.now().isoformat()
    }), 404


@app.errorhandler(405)
def method_not_allowed(error):
    return jsonify({
        'success': False,
        'error': '方法不允许',
        'message': f'该端点不支持 {request.method} 方法',
        'timestamp': datetime.now().isoformat()
    }), 405


@app.errorhandler(500)
def internal_error(error):
    return jsonify({
        'success': False,
        'error': '服务器内部错误',
        'message': '处理请求时发生错误',
        'timestamp': datetime.now().isoformat()
    }), 500


if __name__ == '__main__':
    # 确保必要的目录存在
    os.makedirs('static', exist_ok=True)
    os.makedirs('static/css', exist_ok=True)
    os.makedirs('static/js', exist_ok=True)
    os.makedirs('static/assets', exist_ok=True)

    print("=" * 60)
    print("文本处理API服务")
    print("=" * 60)
    print(f"启动时间: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    print("访问地址: http://127.0.0.1:5000")
    print("\n可用端点:")
    print("  - GET  /                    前端页面")
    print("  - POST /api/process         处理文本")
    print("  - GET  /api/health          健康检查")
    print("  - GET  /api/test            测试接口")
    print("  - POST /api/analyze         文本分析")
    print("=" * 60)

    app.run(host='127.0.0.1', port=5000, debug=True)
