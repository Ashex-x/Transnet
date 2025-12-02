/**
 * 主应用逻辑
 */

// DOM元素
const elements = {
    inputText: document.getElementById('inputText'),
    outputText: document.getElementById('outputText'),
    processBtn: document.getElementById('processBtn'),
    clearBtn: document.getElementById('clearBtn'),
    copyBtn: document.getElementById('copyBtn'),
    charCount: document.getElementById('charCount'),
    apiEndpoint: document.getElementById('apiEndpoint'),
    requestMethod: document.getElementById('requestMethod'),
    statusLight: document.getElementById('statusLight'),
    statusText: document.getElementById('statusText'),
    statusCode: document.getElementById('statusCode'),
    responseTime: document.getElementById('responseTime'),
    dataSize: document.getElementById('dataSize')
};

// 应用状态
const appState = {
    isProcessing: false,
    lastResponse: null,
    charLimit: 1000
};

// 初始化应用
function initApp() {
    console.log('应用初始化...');
    
    // 事件监听器
    setupEventListeners();
    
    // 更新字符计数
    updateCharCount();
    
    // 恢复保存的API设置
    loadSettings();
    
    // 显示初始状态
    setStatus('ready', '就绪');
    
    console.log('应用初始化完成');
}

// 设置事件监听器
function setupEventListeners() {
    // 处理按钮点击
    elements.processBtn.addEventListener('click', handleProcessClick);
    
    // 清空按钮点击
    elements.clearBtn.addEventListener('click', handleClearClick);
    
    // 复制按钮点击
    elements.copyBtn.addEventListener('click', handleCopyClick);
    
    // 输入框输入事件
    elements.inputText.addEventListener('input', handleInputChange);
    
    // API设置变更事件
    elements.apiEndpoint.addEventListener('change', saveSettings);
    elements.requestMethod.addEventListener('change', saveSettings);
    
    // 键盘快捷键
    document.addEventListener('keydown', handleKeydown);
    
    // 页面卸载前保存设置
    window.addEventListener('beforeunload', saveSettings);
}

// 处理文本处理
async function handleProcessClick() {
    if (appState.isProcessing) {
        showToast('正在处理中，请稍候...', 'info');
        return;
    }
    
    const inputText = elements.inputText.value.trim();
    if (!inputText) {
        showToast('请输入要处理的文本', 'error');
        return;
    }
    
    // 开始处理
    startProcessing();
    
    try {
        // 获取API设置
        const endpoint = elements.apiEndpoint.value || '/api/process';
        const method = elements.requestMethod.value;
        
        // 准备请求数据
        const requestData = {
            text: inputText,
            timestamp: new Date().toISOString(),
            options: {
                length: inputText.length,
                language: 'zh-CN'
            }
        };
        
        console.log(`调用API: ${method} ${endpoint}`);
        
        // 发送请求
        const response = await apiClient.request(endpoint, method, requestData);
        
        // 处理响应
        handleApiResponse(response);
        
    } catch (error) {
        console.error('处理失败:', error);
        handleApiError(error);
    } finally {
        finishProcessing();
    }
}

// 处理API响应
function handleApiResponse(response) {
    // 更新状态指示器
    if (response.success) {
        setStatus('ready', '成功');
        showToast('处理成功', 'success');
    } else {
        setStatus('error', '失败');
        showToast(`处理失败: ${response.status}`, 'error');
    }
    
    // 更新响应信息
    elements.statusCode.textContent = response.status;
    elements.responseTime.textContent = `${response.responseTime} ms`;
    
    // 计算数据大小
    const dataSize = JSON.stringify(response.data).length;
    elements.dataSize.textContent = `${dataSize} B`;
    
    // 更新输出
    updateOutput(response);
    
    // 保存响应记录
    appState.lastResponse = response;
    
    // 保存到本地存储（可选）
    saveResponseHistory(response);
}

// 处理API错误
function handleApiError(error) {
    setStatus('error', '错误');
    showToast(`请求失败: ${error.message}`, 'error');
    
    // 显示错误信息
    elements.outputText.textContent = JSON.stringify({
        error: '请求失败',
        message: error.message,
        timestamp: new Date().toISOString()
    }, null, 2);
    
    elements.statusCode.textContent = '0';
    elements.responseTime.textContent = '- ms';
    elements.dataSize.textContent = '- B';
}

// 开始处理
function startProcessing() {
    appState.isProcessing = true;
    setStatus('processing', '处理中...');
    elements.processBtn.disabled = true;
    elements.processBtn.innerHTML = '<i class="fas fa-spinner fa-spin"></i> 处理中...';
    
    // 显示加载状态
    showLoading(true);
}

// 完成处理
function finishProcessing() {
    appState.isProcessing = false;
    elements.processBtn.disabled = false;
    elements.processBtn.innerHTML = '<i class="fas fa-play"></i> 处理文本';
    
    // 隐藏加载状态
    showLoading(false);
}

// 清空输入输出
function handleClearClick() {
    if (confirm('确定要清空所有内容吗？')) {
        elements.inputText.value = '';
        elements.outputText.textContent = '';
        updateCharCount();
        showToast('已清空', 'info');
    }
}

// 复制输出内容
function handleCopyClick() {
    const outputText = elements.outputText.textContent;
    if (!outputText) {
        showToast('没有内容可复制', 'error');
        return;
    }
    
    navigator.clipboard.writeText(outputText)
        .then(() => {
            showToast('已复制到剪贴板', 'success');
            elements.copyBtn.innerHTML = '<i class="fas fa-check"></i> 已复制';
            setTimeout(() => {
                elements.copyBtn.innerHTML = '<i class="fas fa-copy"></i> 复制';
            }, 2000);
        })
        .catch(err => {
            console.error('复制失败:', err);
            showToast('复制失败', 'error');
        });
}

// 处理输入变化
function handleInputChange() {
    updateCharCount();
}

// 更新字符计数
function updateCharCount() {
    const text = elements.inputText.value;
    const count = text.length;
    const remaining = appState.charLimit - count;
    
    elements.charCount.textContent = count;
    
    // 如果超过限制，添加警告样式
    if (count > appState.charLimit) {
        elements.charCount.style.color = '#dc3545';
        elements.charCount.textContent = `${count} (超过限制)`;
    } else if (remaining < 100) {
        elements.charCount.style.color = '#fd7e14';
    } else {
        elements.charCount.style.color = '';
    }
}

// 设置状态
function setStatus(type, message) {
    elements.statusLight.className = 'status-light';
    elements.statusLight.classList.add(type);
    elements.statusText.textContent = message;
}

// 显示加载状态
function showLoading(show) {
    const loadingElement = document.querySelector('.loading-spinner') || createLoadingElement();
    
    if (show) {
        loadingElement.style.display = 'block';
    } else {
        loadingElement.style.display = 'none';
    }
}

// 创建加载元素
function createLoadingElement() {
    const loadingDiv = document.createElement('div');
    loadingDiv.className = 'loading-spinner';
    loadingDiv.innerHTML = `
        <div class="spinner"></div>
        <p>正在处理，请稍候...</p>
    `;
    document.querySelector('.output-section').appendChild(loadingDiv);
    return loadingDiv;
}

// 显示通知
function showToast(message, type = 'info') {
    // 移除现有的通知
    const existingToasts = document.querySelectorAll('.toast');
    existingToasts.forEach(toast => toast.remove());
    
    // 创建新通知
    const toast = document.createElement('div');
    toast.className = `toast ${type}`;
    toast.textContent = message;
    document.body.appendChild(toast);
    
    // 显示通知
    setTimeout(() => toast.classList.add('show'), 10);
    
    // 自动隐藏
    setTimeout(() => {
        toast.classList.remove('show');
        setTimeout(() => toast.remove(), 300);
    }, 3000);
}

// 更新输出
function updateOutput(response) {
    let outputContent;
    
    if (response.success) {
        outputContent = response.data;
    } else {
        outputContent = {
            error: response.data.error || '未知错误',
            message: response.data.message,
            status: response.status
        };
    }
    
    // 格式化输出
    if (typeof outputContent === 'object') {
        elements.outputText.textContent = JSON.stringify(outputContent, null, 2);
    } else {
        elements.outputText.textContent = outputContent;
    }
    
    // 添加样式类
    elements.outputText.classList.add('fade-in');
    setTimeout(() => elements.outputText.classList.remove('fade-in'), 500);
}

// 处理键盘快捷键
function handleKeydown(e) {
    // Ctrl/Cmd + Enter 处理文本
    if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
        e.preventDefault();
        handleProcessClick();
    }
    
    // Ctrl/Cmd + C 复制输出
    if ((e.ctrlKey || e.metaKey) && e.key === 'c') {
        const activeElement = document.activeElement;
        if (activeElement !== elements.inputText) {
            e.preventDefault();
            handleCopyClick();
        }
    }
    
    // Escape 清空
    if (e.key === 'Escape') {
        handleClearClick();
    }
}

// 保存设置到本地存储
function saveSettings() {
    const settings = {
        apiEndpoint: elements.apiEndpoint.value,
        requestMethod: elements.requestMethod.value
    };
    
    try {
        localStorage.setItem('apiSettings', JSON.stringify(settings));
    } catch (error) {
        console.warn('无法保存设置到本地存储:', error);
    }
}

// 从本地存储加载设置
function loadSettings() {
    try {
        const savedSettings = localStorage.getItem('apiSettings');
        if (savedSettings) {
            const settings = JSON.parse(savedSettings);
            
            if (settings.apiEndpoint) {
                elements.apiEndpoint.value = settings.apiEndpoint;
            }
            
            if (settings.requestMethod) {
                elements.requestMethod.value = settings.requestMethod;
            }
        }
    } catch (error) {
        console.warn('无法从本地存储加载设置:', error);
    }
}

// 保存响应历史（可选）
function saveResponseHistory(response) {
    try {
        let history = JSON.parse(localStorage.getItem('apiHistory') || '[]');
        
        const historyEntry = {
            timestamp: new Date().toISOString(),
            endpoint: elements.apiEndpoint.value,
            method: elements.requestMethod.value,
            status: response.status,
            responseTime: response.responseTime,
            inputLength: elements.inputText.value.length
        };
        
        history.unshift(historyEntry); // 添加到开头
        
        // 只保留最近的20条记录
        if (history.length > 20) {
            history = history.slice(0, 20);
        }
        
        localStorage.setItem('apiHistory', JSON.stringify(history));
    } catch (error) {
        console.warn('无法保存响应历史:', error);
    }
}

// 页面加载完成后初始化应用
document.addEventListener('DOMContentLoaded', initApp);

// 导出函数供其他脚本使用（如果需要）
if (typeof module !== 'undefined' && module.exports) {
    module.exports = {
        initApp,
        handleProcessClick,
        handleClearClick,
        updateCharCount
    };
}