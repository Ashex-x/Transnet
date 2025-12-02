/**
 * API客户端模块 - 负责与后端API通信
 */

class APIClient {
    constructor(baseURL = '') {
        this.baseURL = baseURL;
        this.defaultHeaders = {
            'Content-Type': 'application/json',
            'Accept': 'application/json'
        };
    }

    /**
     * 设置基础URL
     * @param {string} baseURL - API基础URL
     */
    setBaseURL(baseURL) {
        this.baseURL = baseURL;
    }

    /**
     * 设置默认请求头
     * @param {Object} headers - 请求头对象
     */
    setDefaultHeaders(headers) {
        this.defaultHeaders = { ...this.defaultHeaders, ...headers };
    }

    /**
     * 发送HTTP请求
     * @param {string} endpoint - API端点
     * @param {string} method - HTTP方法
     * @param {Object} data - 请求数据
     * @param {Object} headers - 额外请求头
     * @returns {Promise<Object>} - 响应数据
     */
    async request(endpoint, method = 'GET', data = null, headers = {}) {
        const url = endpoint.startsWith('http') ? endpoint : `${this.baseURL}${endpoint}`;
        const options = {
            method: method.toUpperCase(),
            headers: { ...this.defaultHeaders, ...headers },
            credentials: 'same-origin' // 根据需求调整
        };

        if (data && (method.toUpperCase() === 'POST' || method.toUpperCase() === 'PUT')) {
            options.body = JSON.stringify(data);
        } else if (data && method.toUpperCase() === 'GET') {
            // 为GET请求添加查询参数
            const params = new URLSearchParams(data);
            endpoint = `${endpoint}?${params.toString()}`;
        }

        try {
            const startTime = Date.now();
            const response = await fetch(url, options);
            const responseTime = Date.now() - startTime;

            const responseData = await this.handleResponse(response);
            
            return {
                success: response.ok,
                status: response.status,
                data: responseData,
                responseTime: responseTime,
                headers: response.headers
            };
        } catch (error) {
            return this.handleError(error);
        }
    }

    /**
     * 处理响应
     * @param {Response} response - fetch响应对象
     * @returns {Promise<Object>} - 响应数据
     */
    async handleResponse(response) {
        const contentType = response.headers.get('content-type');
        
        if (contentType && contentType.includes('application/json')) {
            return await response.json();
        } else if (contentType && contentType.includes('text/')) {
            return await response.text();
        } else {
            return await response.blob();
        }
    }

    /**
     * 处理错误
     * @param {Error} error - 错误对象
     * @returns {Object} - 错误响应
     */
    handleError(error) {
        console.error('API请求失败:', error);
        
        return {
            success: false,
            status: 0,
            data: {
                error: '网络错误',
                message: error.message || '无法连接到服务器',
                details: error.toString()
            },
            responseTime: 0
        };
    }

    /**
     * GET请求简写方法
     * @param {string} endpoint - API端点
     * @param {Object} params - 查询参数
     * @param {Object} headers - 额外请求头
     */
    async get(endpoint, params = {}, headers = {}) {
        return this.request(endpoint, 'GET', params, headers);
    }

    /**
     * POST请求简写方法
     * @param {string} endpoint - API端点
     * @param {Object} data - 请求数据
     * @param {Object} headers - 额外请求头
     */
    async post(endpoint, data = {}, headers = {}) {
        return this.request(endpoint, 'POST', data, headers);
    }

    /**
     * PUT请求简写方法
     * @param {string} endpoint - API端点
     * @param {Object} data - 请求数据
     * @param {Object} headers - 额外请求头
     */
    async put(endpoint, data = {}, headers = {}) {
        return this.request(endpoint, 'PUT', data, headers);
    }

    /**
     * DELETE请求简写方法
     * @param {string} endpoint - API端点
     * @param {Object} data - 请求数据
     * @param {Object} headers - 额外请求头
     */
    async delete(endpoint, data = {}, headers = {}) {
        return this.request(endpoint, 'DELETE', data, headers);
    }

    /**
     * 上传文件
     * @param {string} endpoint - API端点
     * @param {File} file - 文件对象
     * @param {Object} additionalData - 额外数据
     */
    async uploadFile(endpoint, file, additionalData = {}) {
        const formData = new FormData();
        formData.append('file', file);
        
        Object.keys(additionalData).forEach(key => {
            formData.append(key, additionalData[key]);
        });

        const headers = {
            ...this.defaultHeaders,
            'Content-Type': undefined // 让浏览器设置正确的Content-Type
        };

        return this.request(endpoint, 'POST', formData, headers);
    }

    /**
     * 设置认证token
     * @param {string} token - 认证token
     */
    setAuthToken(token) {
        this.defaultHeaders['Authorization'] = `Bearer ${token}`;
    }

    /**
     * 清除认证token
     */
    clearAuthToken() {
        delete this.defaultHeaders['Authorization'];
    }
}

// 创建全局API客户端实例
const apiClient = new APIClient();

// 导出API客户端
if (typeof module !== 'undefined' && module.exports) {
    module.exports = { APIClient, apiClient };
}