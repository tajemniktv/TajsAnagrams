"""Named native-policy adaptations; never edit the pinned oracle or drop fields."""
import inspect

import anagram_comparative_grammar as comparative
import anagram_rerank_core as core


def reviewed_grammar():
    """The three reviewed native corrections, shared by every grammar consumer."""
    if getattr(core, '_native_policy_v1', False):
        return
    core._comparative_like = lambda word, lex: comparative.comparative_evidence(word, lex).confidence >= 0.9
    edits = {
        '_comparative_span_starting_at': ('or any(lex.features(w).adj or lex.features(w).adv for w in left)', ''),
        '_np_span_starting_at': ('and words[i].endswith(("ed", "en")) and following_is_nominal', 'and function_class(words[i]) is None and words[i].endswith(("ed", "en")) and following_is_nominal'),
    }
    for name, (before, after) in edits.items():
        source = inspect.getsource(getattr(core, name))
        if before not in source:
            raise RuntimeError(f'Reference correction no longer applies: {name}')
        exec(compile(source.replace(before, after), f'<native-policy-v1:{name}>', 'exec'), core.__dict__)
    core._native_policy_v1 = True


def native_rows(value, *, display=False):
    """Add the exact expected native presentation field, retaining strict schemas."""
    if isinstance(value, dict):
        result = {key: native_rows(item, display=display) for key, item in value.items()}
        if 'best_order' in result and 'word_count' in result:
            result['display_phrase'] = core.pretty_phrase(result['best_order']) if display else ''
        return result
    if isinstance(value, (list, tuple)):
        return [native_rows(item, display=display) for item in value]
    return value
