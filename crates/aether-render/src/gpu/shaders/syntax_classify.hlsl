// 语法分类 Shader
// 基于 Token 流识别简单语法模式

#define THREAD_GROUP_SIZE 256
#define MAX_PATTERN_LENGTH 4

// Token 类型常量
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

// 语法分类常量
#define SYNTAX_UNKNOWN          0
#define SYNTAX_FUNCTION_DECL    1
#define SYNTAX_FUNCTION_CALL    2
#define SYNTAX_TYPE_NAME        3
#define SYNTAX_VARIABLE_DECL    4
#define SYNTAX_VARIABLE_REF     5
#define SYNTAX_PARAMETER        6
#define SYNTAX_FIELD_ACCESS     7
#define SYNTAX_MACRO            8
#define SYNTAX_ATTRIBUTE        9
#define SYNTAX_LIFETIME         10
#define SYNTAX_MODULE           11
#define SYNTAX_TRAIT            12
#define SYNTAX_STRUCT           13
#define SYNTAX_ENUM             14
#define SYNTAX_IMPL             15

// Token 结构
struct Token {
    uint start;
    uint len;
    uint token_type;
    uint keyword_id;
    uint syntax_class;
    uint _padding;
};

// 语法模式结构
struct SyntaxPattern {
    uint pattern_type;
    uint4 token_sequence;
    uint sequence_len;
    uint output_class;
    uint priority;
    uint2 _padding;
};

// 输入 Token 缓冲区
StructuredBuffer<Token> Tokens : register(t0);
// 语法模式缓冲区
StructuredBuffer<SyntaxPattern> Patterns : register(t1);

// 输出语法分类
RWStructuredBuffer<uint4> SyntaxClasses : register(u0);
// output_class, confidence, _padding[2]

// 常量缓冲区
cbuffer SyntaxConstants : register(b0) {
    uint TokenCount;
    uint PatternCount;
    uint _padding[2];
};

// 匹配模式
bool MatchPattern(uint token_idx, SyntaxPattern pattern, out uint match_len) {
    if (token_idx + pattern.sequence_len > TokenCount) {
        match_len = 0;
        return false;
    }
    
    for (uint i = 0; i < pattern.sequence_len; i++) {
        Token tok = Tokens[token_idx + i];
        uint expected = 0;
        
        switch (i) {
            case 0: expected = pattern.token_sequence.x; break;
            case 1: expected = pattern.token_sequence.y; break;
            case 2: expected = pattern.token_sequence.z; break;
            case 3: expected = pattern.token_sequence.w; break;
        }
        
        if (tok.token_type != expected) {
            match_len = 0;
            return false;
        }
    }
    
    match_len = pattern.sequence_len;
    return true;
}

[numthreads(THREAD_GROUP_SIZE, 1, 1)]
void main(uint3 id : SV_DispatchThreadID) {
    uint token_idx = id.x;
    if (token_idx >= TokenCount) return;
    
    Token tok = Tokens[token_idx];
    
    // 只处理标识符和关键字
    if (tok.token_type != TOKEN_IDENTIFIER && tok.token_type != TOKEN_KEYWORD) {
        return;
    }
    
    uint best_class = SYNTAX_UNKNOWN;
    uint best_priority = 0;
    uint best_match_len = 0;
    
    // 尝试匹配所有模式
    for (uint p = 0; p < PatternCount; p++) {
        SyntaxPattern pattern = Patterns[p];
        uint match_len = 0;
        
        if (MatchPattern(token_idx, pattern, match_len)) {
            if (pattern.priority > best_priority) {
                best_priority = pattern.priority;
                best_class = pattern.output_class;
                best_match_len = match_len;
            }
        }
    }
    
    // 写入结果
    if (best_class != SYNTAX_UNKNOWN) {
        uint confidence = 70 + (best_priority / 2); // 基础置信度 + 优先级加成
        if (confidence > 100) confidence = 100;
        
        SyntaxClasses[token_idx] = uint4(best_class, confidence, best_match_len, 0);
        
        // 更新 Token 的语法分类（可选）
        // 注意：这需要 UAV 访问 Tokens 缓冲区
    }
}
