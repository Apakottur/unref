BEFORE_SHADOW = 1
BEFORE_SHADOW_IN_FUNCTION = 2
COUNTER = 0  # expect: 1 [unused-variable]
WITH_VARS = 0  # expect: 1 [unused-variable]
WITH_LOCALS = 0  # expect: 1 [unused-variable]

globals()["BEFORE_SHADOW"]


def read_before_shadow():
    return globals()["BEFORE_SHADOW_IN_FUNCTION"]


read_before_shadow()


def globals():
    return {"COUNTER": 5}


vars = lambda: {"WITH_VARS": 5}
locals = lambda: {"WITH_LOCALS": 5}

globals()["COUNTER"] += 1
globals()["COUNTER"], vars().get("WITH_VARS"), "WITH_LOCALS" in locals()
