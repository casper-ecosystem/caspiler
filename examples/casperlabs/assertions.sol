contract Contract {

	function assertFn(int32 a) public {
        assert(a > 0);
	}

	function revertFn() public {
        revert("This is ignored");
	}

	function revertWithString() public {
        revert("This is ignored");
	}

	function requireFn(int32 a) public {
        require(a > 0);
	}

	function requireWithString(int32 a) public {
        require(a > 0, "This is ignored");
	}
}
