from .provider import Service as API
from .provider import ShadowReplacement, ShadowSource as Adjusted
from .provider import ReboundSource as Rebound
from .provider import ReboundReplacement as Rebound
from .provider import LoopReplacement, LoopSource as Looped

Adjusted = ShadowReplacement

for Looped in [LoopReplacement]:
    pass
