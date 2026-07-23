#!/usr/bin/env python3
"""Validate the emitted contract with a third-party JSON Schema engine.

`cargo test` asserts what the schema *claims* — that fields are documented, that
closed sets match the Rust enums. This asserts what a foreign engine *does* with
it, so the schema is not marked by the code that produced it.

CI runs this file directly, and so can you:

    python3 dev/conformance.py                     # check the published example
    python3 dev/conformance.py /tmp/example.json   # and a freshly emitted one

It lives here rather than inside the workflow so there is one copy: an embedded
copy drifts the moment the model changes, and a workflow that has drifted only
tells you on push.
"""
import json
import sys

from jsonschema import Draft202012Validator as Validator, ValidationError

SCHEMA_PATH = "schema/argenv-contract.v1.schema.json"


def main(examples: list[str]) -> int:
    schema = json.load(open(SCHEMA_PATH))
    Validator.check_schema(schema)
    print("the schema is a valid draft 2020-12 schema")

    for path in examples:
        Validator(schema).validate(json.load(open(path)))
        print(f"{path} validates")

    doc = json.load(open(examples[0]))

    def first_with(binding: str) -> int:
        """Index of the first input declaring the given binding."""
        return next(n for n, r in enumerate(doc["inputs"]) if binding in r)

    def corrupt(mutate) -> dict:
        bad = json.loads(json.dumps(doc))
        mutate(bad["inputs"])
        return bad

    rejected = {
        "unknown type": lambda i: i[0].__setitem__("type", "nonsense"),
        "unknown stability": lambda i: i[0].__setitem__("stability", "sorta"),
        "malformed version": lambda i: i[0].__setitem__("since", "1.x"),
        "malformed date": lambda i: i[0].__setitem__("reviewed", "2026-13-45"),
        "non-snake-case key": lambda i: i[0].__setitem__("key", "Not A Key"),
        "unsettable variable name":
            lambda i: i[first_with("env")].__setitem__("env", {"name": "BAD NAME"}),
        "long flag carrying dashes":
            lambda i: i[first_with("arg")]["arg"].__setitem__("long", "--dashed"),
    }

    for label, mutate in rejected.items():
        try:
            Validator(schema).validate(corrupt(mutate))
        except ValidationError:
            print(f"rejected: {label}")
        else:
            print(f"schema failed to reject: {label}", file=sys.stderr)
            return 1

    # A field from a later version must not invalidate the document: the format
    # grows by adding optional fields.
    forward = json.loads(json.dumps(doc))
    forward["inputs"][0]["field_from_a_later_version"] = "x"
    Validator(schema).validate(forward)
    print("a document from a later version still validates")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:] or ["api/v1/example.json"]))
