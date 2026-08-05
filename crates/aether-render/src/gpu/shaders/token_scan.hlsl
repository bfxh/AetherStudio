// Phase 2: Token 扫描 Shader
// 基于字符分类结果，识别 Token 边界

#define THREAD_GROUP_SIZE 256

// Token 类型常量（与 Rust 侧对齐）
#define TOKEN_UNKNOWN       0
#define TOKEN_IDENTIFIER    1
#define TOKEN_KEYWORD       2
#define TOKEN_STRING        3
#define TOKEN_NUMBER        4
#define TOKEN_COMMENT       5
#define TOKEN_OPERATOR      6
#define TOKEN_PUNCTUATION   7
#define TOKEN_WHITESPACE    8
#define TOKEN_NEWLINE       9
#define TOKEN_PREPROCESSOR  10

// 字符分类常量（与 char_classify.hlsl 对齐）
#define CHAR_LETTER       1
#define CHAR_DIGIT        2
#define CHAR_UNDERSCORE   3
#define CHAR_SPACE        4
#define CHAR_TAB          5
#define CHAR_NEWLINE      6
#define CHAR_QUOTE_SINGLE 7
#define CHAR_QUOTE_DOUBLE 8
#define CHAR_SLASH        9
#define CHAR_STAR         10
#define CHAR_HASH         11
#define CHAR_LPAREN       12
#define CHAR_RPAREN       13
#define CHAR_LBRACE       14
#define CHAR_RBRACE       15
#define CHAR_LBRACKET     16
#define CHAR_RBRACKET     17
#define CHAR_SEMICOLON    18
#define CHAR_COLON        19
#define CHAR_COMMA        20
#define CHAR_DOT          21
#define CHAR_PLUS         22
#define CHAR_MINUS        23
#define CHAR_EQUAL        24
#define CHAR_BANG         25
#define CHAR_LESS         26
#define CHAR_GREATER      27
#define CHAR_AMPERSAND    28
#define CHAR_PIPE         29
#define CHAR_PERCENT      30
#define CHAR_CARET        31
#define CHAR_TILDE        32
#define CHAR_AT           33
#define CHAR_DOLLAR       34
#define CHAR_BACKSLASH    35

// Token 结构（与 Rust 侧 GpuToken 对齐）
struct Token {
    uint start;
    uint len;
    uint token_type;
    uint keyword_id;
    uint syntax_class;
    uint _padding;
};

// 输入：字符分类结果
StructuredBuffer<uint> CharClasses : register(t0);
// 输出：Token 列表
RWStructuredBuffer<Token> Tokens : register(u0);
// 输出：Token 计数（带原子计数器的 UAV）
RWStructuredBuffer<uint> TokenCount : register(u1);

// 常量缓冲区
cbuffer LexerConstants : register(b0) {
    uint TextLength;
    uint MaxTokens;
    uint NumStates;
    uint KeywordTableSize;
};

// 组共享内存：标记 token 起始位置
groupshared uint local_starts[THREAD_GROUP_SIZE];
groupshared uint local_types[THREAD_GROUP_SIZE];

// 判断字符是否属于标识符
bool IsIdentifierChar(uint char_class) {
    return char_class == CHAR_LETTER || 
           char_class == CHAR_DIGIT || 
           char_class == CHAR_UNDERSCORE;
}

// 判断字符是否是数字的一部分
bool IsNumberChar(uint char_class, uint prev_class) {
    return char_class == CHAR_DIGIT || 
           (char_class == CHAR_DOT && prev_class == CHAR_DIGIT) ||
           (char_class == CHAR_UNDERSCORE && prev_class == CHAR_DIGIT);
}

// 判断字符是否是空白
bool IsWhitespace(uint char_class) {
    return char_class == CHAR_SPACE || char_class == CHAR_TAB;
}

// 判断字符是否是换行
bool IsNewline(uint char_class) {
    return char_class == CHAR_NEWLINE;
}

// 判断字符是否是标点
bool IsPunctuation(uint char_class) {
    return char_class >= CHAR_LPAREN && char_class <= CHAR_COMMA;
}

// 判断字符是否是运算符开始
bool IsOperatorStart(uint char_class) {
    return char_class == CHAR_PLUS || char_class == CHAR_MINUS ||
           char_class == CHAR_STAR || char_class == CHAR_SLASH ||
           char_class == CHAR_PERCENT || char_class == CHAR_EQUAL ||
           char_class == CHAR_BANG || char_class == CHAR_LESS ||
           char_class == CHAR_GREATER || char_class == CHAR_AMPERSAND ||
           char_class == CHAR_PIPE || char_class == CHAR_CARET ||
           char_class == CHAR_TILDE || char_class == CHAR_DOT;
}

// 获取单字符 token 类型
uint GetSingleCharTokenType(uint char_class) {
    switch (char_class) {
        case CHAR_LPAREN: case CHAR_RPAREN:
        case CHAR_LBRACE: case CHAR_RBRACE:
        case CHAR_LBRACKET: case CHAR_RBRACKET:
        case CHAR_SEMICOLON: case CHAR_COLON:
        case CHAR_COMMA:
            return TOKEN_PUNCTUATION;
        case CHAR_NEWLINE:
            return TOKEN_NEWLINE;
        default:
            return TOKEN_UNKNOWN;
    }
}

[numthreads(THREAD_GROUP_SIZE, 1, 1)]
void main(uint3 id : SV_DispatchThreadID, uint3 group_id : SV_GroupID) {
    uint idx = id.x;
    uint local_idx = id.x % THREAD_GROUP_SIZE;
    
    // 初始化共享内存
    local_starts[local_idx] = 0;
    local_types[local_idx] = TOKEN_UNKNOWN;
    
    GroupMemoryBarrierWithGroupSync();
    
    if (idx >= TextLength) return;
    
    uint char_class = CharClasses[idx];
    uint prev_class = (idx > 0) ? CharClasses[idx - 1] : 0;
    
    // 判断是否是 token 起始位置
    bool is_token_start = false;
    uint token_type = TOKEN_UNKNOWN;
    
    if (idx == 0) {
        // 文本开始，总是 token 起始
        is_token_start = true;
    } else if (IsWhitespace(char_class)) {
        // 空白字符：合并为单个 whitespace token
        if (!IsWhitespace(prev_class)) {
            is_token_start = true;
            token_type = TOKEN_WHITESPACE;
        }
    } else if (IsNewline(char_class)) {
        // 换行：单独一个 token
        if (!IsNewline(prev_class)) {
            is_token_start = true;
            token_type = TOKEN_NEWLINE;
        }
    } else if (char_class == CHAR_QUOTE_DOUBLE || char_class == CHAR_QUOTE_SINGLE) {
        // 字符串开始
        is_token_start = true;
        token_type = TOKEN_STRING;
    } else if (char_class == CHAR_SLASH && idx + 1 < TextLength) {
        // 可能是注释开始
        uint next_class = CharClasses[idx + 1];
        if (next_class == CHAR_SLASH || next_class == CHAR_STAR) {
            is_token_start = true;
            token_type = TOKEN_COMMENT;
        } else {
            is_token_start = !IsOperatorStart(prev_class);
            token_type = TOKEN_OPERATOR;
        }
    } else if (char_class == CHAR_HASH && idx == 0) {
        // 预处理指令
        is_token_start = true;
        token_type = TOKEN_PREPROCESSOR;
    } else if (IsIdentifierChar(char_class)) {
        // 标识符
        if (!IsIdentifierChar(prev_class)) {
            is_token_start = true;
            token_type = TOKEN_IDENTIFIER;
        }
    } else if (IsNumberChar(char_class, prev_class)) {
        // 数字
        if (!IsNumberChar(prev_class, (idx > 1) ? CharClasses[idx - 2] : 0)) {
            is_token_start = true;
            token_type = TOKEN_NUMBER;
        }
    } else if (IsOperatorStart(char_class)) {
        // 运算符
        if (!IsOperatorStart(prev_class)) {
            is_token_start = true;
            token_type = TOKEN_OPERATOR;
        }
    } else if (IsPunctuation(char_class)) {
        // 标点
        is_token_start = true;
        token_type = TOKEN_PUNCTUATION;
    }
    
    // 标记 token 起始
    if (is_token_start) {
        local_starts[local_idx] = 1;
        local_types[local_idx] = token_type;
    }
    
    GroupMemoryBarrierWithGroupSync();
    
    // 前缀和计算 token 索引（简化版：只处理组内）
    // 注意：完整实现需要跨组前缀和
    if (local_idx == 0) {
        uint token_idx = 0;
        for (uint i = 0; i < THREAD_GROUP_SIZE; i++) {
            if (local_starts[i] == 1) {
                uint global_idx = group_id.x * THREAD_GROUP_SIZE + i;
                if (global_idx < TextLength) {
                    // 计算 token 长度（到下一个 token 开始或文本结束）
                    uint token_start = global_idx;
                    uint token_len = 1;
                    uint j = global_idx + 1;
                    
                    // 根据 token 类型决定如何扩展
                    uint ttype = local_types[i];
                    
                    if (ttype == TOKEN_STRING) {
                        // 字符串：找到匹配的引号
                        uint quote_char = CharClasses[token_start];
                        j = token_start + 1;
                        while (j < TextLength && CharClasses[j] != quote_char) {
                            if (CharClasses[j] == CHAR_BACKSLASH) j++; // 转义
                            j++;
                        }
                        if (j < TextLength) j++; // 包含结束引号
                        token_len = j - token_start;
                    } else if (ttype == TOKEN_COMMENT) {
                        // 注释：到行尾或 */
                        if (token_start + 1 < TextLength && CharClasses[token_start + 1] == CHAR_STAR) {
                            // 块注释 /* */
                            j = token_start + 2;
                            while (j + 1 < TextLength && !(CharClasses[j] == CHAR_STAR && CharClasses[j + 1] == CHAR_SLASH)) {
                                j++;
                            }
                            if (j + 1 < TextLength) j += 2;
                        } else {
                            // 行注释 //
                            j = token_start + 2;
                            while (j < TextLength && !IsNewline(CharClasses[j])) {
                                j++;
                            }
                        }
                        token_len = j - token_start;
                    } else if (ttype == TOKEN_WHITESPACE) {
                        while (j < TextLength && IsWhitespace(CharClasses[j])) {
                            j++;
                        }
                        token_len = j - token_start;
                    } else if (ttype == TOKEN_IDENTIFIER) {
                        while (j < TextLength && IsIdentifierChar(CharClasses[j])) {
                            j++;
                        }
                        token_len = j - token_start;
                    } else if (ttype == TOKEN_NUMBER) {
                        while (j < TextLength && IsNumberChar(CharClasses[j], CharClasses[j - 1])) {
                            j++;
                        }
                        token_len = j - token_start;
                    } else if (ttype == TOKEN_OPERATOR) {
                        while (j < TextLength && IsOperatorStart(CharClasses[j])) {
                            j++;
                        }
                        token_len = j - token_start;
                    } else {
                        // 单字符 token
                        token_len = 1;
                    }
                    
                    // 原子递增获取全局 token 索引
                    uint global_token_idx;
                    InterlockedAdd(TokenCount[0], 1, global_token_idx);
                    
                    if (global_token_idx < MaxTokens) {
                        Token tok;
                        tok.start = token_start;
                        tok.len = token_len;
                        tok.token_type = ttype;
                        tok.keyword_id = 0;
                        tok.syntax_class = 0;
                        tok._padding = 0;
                        Tokens[global_token_idx] = tok;
                    }
                }
            }
        }
    }
}
