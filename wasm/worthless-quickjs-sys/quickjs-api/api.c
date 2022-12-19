#include "api.h"

const JSValue WL_JS_NULL = JS_NULL;
const JSValue WL_JS_UNDEFINED = JS_UNDEFINED;
const JSValue WL_JS_TRUE = JS_TRUE;

int WL_GetRefCount(JSValue value)
{
    if (JS_VALUE_HAS_REF_COUNT(value)) {
        JSRefCountHeader *p = (JSRefCountHeader *)JS_VALUE_GET_PTR(value);
        return p->ref_count;
    } else {
        return 0;
    }
}

JSValue WL_JS_DupValue(JSContext *ctx, JSValueConst v)
{
    return JS_DupValue(ctx, v);
}

void WL_JS_FreeValue(JSContext *ctx, JSValue val)
{
    JS_FreeValue(ctx, val);
}

JSValue WL_JS_NewFloat64(JSContext *ctx, double d)
{
    return __JS_NewFloat64(ctx, d);
}

JSValue WL_JS_NewInt32(JSContext *ctx, int32_t val)
{
    return JS_NewInt32(ctx, val);
}

JSValue WL_JS_NewBool(JSContext *ctx, int32_t val)
{
    return JS_NewBool(ctx, val); 
}