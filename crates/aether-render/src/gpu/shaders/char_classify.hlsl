// Phase 1: 字符分类 Shader
// 每个线程处理 1 个字符，将字符分类为词法类别

#define THREAD_GROUP_SIZE 256

// 字符分类常量
#define CHAR_UNKNOWN      0
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

// 分类查找表（256 字节，可放入共享内存）
static const uint CHAR_CLASS_TABLE[256] = {
    // 控制字符 (0-31)
    0,0,0,0,0,0,0,0,0,5,6,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    // 空格和标点 (32-63)
    4,25,8,11,34,30,28,7,12,13,10,22,20,23,21,9,2,2,2,2,2,2,2,2,2,2,19,18,26,24,27,0,
    // @ A-Z (64-95)
    33,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,16,35,17,32,3,
    // ` a-z (96-127)
    0,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,14,29,15,0,0,
    // 扩展 ASCII (128-255) - 简化为 LETTER 或 UNKNOWN
    1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
    1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
    1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
    1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1
};

// 输入文本缓冲区
StructuredBuffer<uint> TextBuffer : register(t0);
// 输出字符分类
RWStructuredBuffer<uint> CharClasses : register(u0);

// 常量缓冲区
cbuffer LexerConstants : register(b0) {
    uint TextLength;
    uint MaxTokens;
    uint NumStates;
    uint KeywordTableSize;
};

// 辅助函数：从 uint 缓冲区读取第 idx 个字节
uint ReadByte(uint idx) {
    uint word = TextBuffer[idx / 4];
    uint shift = (idx % 4) * 8;
    return (word >> shift) & 0xFF;
}

[numthreads(THREAD_GROUP_SIZE, 1, 1)]
void main(uint3 id : SV_DispatchThreadID) {
    uint idx = id.x;
    if (idx >= TextLength) return;
    
    uint byte = ReadByte(idx);
    CharClasses[idx] = CHAR_CLASS_TABLE[byte];
}
