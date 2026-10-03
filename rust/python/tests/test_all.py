import pytest
import viewfinder_core


def test_sum_as_string():
    assert viewfinder_core.sum_as_string(1, 1) == "2"
