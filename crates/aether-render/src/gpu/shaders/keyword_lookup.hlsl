// Phase 3: 关键字查找 Shader
// 基于完美哈希表识别关键字

#define THREAD_GROUP_SIZE 256

// Token 类型常量
#define TOKEN_IDENTIFIER    1
#define TOKEN_KEYWORD       2

// Token 结构
struct Token {
    uint start;
    uint len;
    uint token_type;
    uint keyword_id;
    uint syntax_class;
    uint _padding;
};

// 输入/输出 Token 缓冲区
RWStructuredBuffer<Token> Tokens : register(u0);

// 关键字哈希表（完美哈希）
StructuredBuffer<uint> KeywordHash : register(t0);

// 常量缓冲区
cbuffer LexerConstants : register(b0) {
    uint TextLength;
    uint MaxTokens;
    uint NumStates;
    uint KeywordTableSize;
};

// 文本缓冲区（用于读取标识符内容）
StructuredBuffer<uint> TextBuffer : register(t1);

// 辅助函数：从 uint 缓冲区读取第 idx 个字节
uint ReadByte(uint idx) {
    uint word = TextBuffer[idx / 4];
    uint shift = (idx % 4) * 8;
    return (word >> shift) & 0xFF;
}

// FNV-1a 哈希函数
uint FNV1aHash(uint start, uint len) {
    const uint FNV_PRIME = 16777619;
    const uint FNV_OFFSET = 2166136261;
    
    uint hash = FNV_OFFSET;
    for (uint i = 0; i < len && i < 64; i++) { // 限制最大长度
        uint byte = ReadByte(start + i);
        // 转换为小写（简化）
        if (byte >= 'A' && byte <= 'Z') {
            byte = byte - 'A' + 'a';
        }
        hash ^= byte;
        hash *= FNV_PRIME;
    }
    return hash;
}

// 完美哈希查找
bool LookupKeyword(uint start, uint len, out uint keyword_id) {
    if (len > 32 || len == 0) { // 关键字长度限制
        keyword_id = 0;
        return false;
    }
    
    uint hash = FNV1aHash(start, len);
    uint idx = hash % KeywordTableSize;
    
    // 检查哈希表
    uint stored_hash = KeywordHash[idx * 2];
    uint stored_id = KeywordHash[idx * 2 + 1];
    
    if (stored_hash == hash && stored_id != 0) {
        keyword_id = stored_id;
        return true;
    }
    
    keyword_id = 0;
    return false;
}

[numthreads(THREAD_GROUP_SIZE, 1, 1)]
void main(uint3 id : SV_DispatchThreadID) {
    uint idx = id.x;
    if (idx >= MaxTokens) return;
    
    Token tok = Tokens[idx];
    
    // 只处理标识符
    if (tok.token_type != TOKEN_IDENTIFIER) return;
    
    uint keyword_id;
    if (LookupKeyword(tok.start, tok.len, keyword_id)) {
        tok.token_type = TOKEN_KEYWORD;
        tok.keyword_id = keyword_id;
        Tokens[idx] = tok;
    }
}
