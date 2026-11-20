import import_rebinding_first as source

source.value()

import import_rebinding_second as source

source.value()

import import_rebinding_first as replacement
import import_rebinding_second as replacement

replacement.current()
