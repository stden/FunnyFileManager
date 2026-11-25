"""Test the Python complexity gate: what it counts, and what it refuses."""

from pathlib import Path
import sys
import textwrap
import unittest

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))
import lint_complexity  # noqa: E402


def function(source):
    return lint_complexity.ast.parse(textwrap.dedent(source)).body[0]


def problems(source):
    return list(lint_complexity.findings(Path("x.py"), textwrap.dedent(source)))


class ComplexityTests(unittest.TestCase):
    def test_a_straight_function_has_one_path(self):
        self.assertEqual(lint_complexity.complexity(function("def f():\n    return 1\n")), 1)

    def test_each_branch_and_extra_operand_adds_a_path(self):
        source = """
            def f(a, b, c):
                if a and b or c:
                    return [x for x in a if x]
                for item in b:
                    try:
                        pass
                    except ValueError:
                        pass
                return 1 if c else 2
        """
        # 1 + if + (and, or) + comprehension + its if + for + except + ternary
        self.assertEqual(lint_complexity.complexity(function(source)), 9)

    def test_a_nested_function_is_measured_on_its_own(self):
        source = """
            def outer():
                def inner(x):
                    if x:
                        return 1
                return inner
        """
        self.assertEqual(lint_complexity.complexity(function(source)), 1)

    def test_blocks_are_counted_as_deep_as_they_nest(self):
        source = """
            def f(a):
                if a:
                    for x in a:
                        while x:
                            with open(x):
                                pass
        """
        self.assertEqual(lint_complexity.nesting(function(source)), 4)

    def test_a_function_over_budget_is_named_with_its_line(self):
        branches = "".join(f"    if a == {n}:\n        return {n}\n" for n in range(11))
        found = problems("def busy(a):\n" + branches)
        self.assertEqual(len(found), 1)
        self.assertIn("x.py:1: busy has cyclomatic complexity 12", found[0])

    def test_nesting_over_budget_is_named(self):
        source = """
            def deep(a):
                if a:
                    if a:
                        if a:
                            if a:
                                if a:
                                    pass
        """
        self.assertTrue(any("nests blocks 5 deep" in problem for problem in problems(source)))

    def test_the_repository_is_within_budget(self):
        self.assertEqual(lint_complexity.main([]), 0)


if __name__ == "__main__":
    unittest.main()
