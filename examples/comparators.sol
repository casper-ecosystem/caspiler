contract Contract {

	function equals(uint256 a, uint256 b) public {
        bool r = a == b;
	}

    function notEquals(uint256 a, uint256 b) public {
        bool r = a != b;
	}

    function more(uint256 a, uint256 b) public {
        bool r = a > b;
	}

    function less(uint256 a, uint256 b) public {
        bool r = a < b;
	}

    function moreEq(uint256 a, uint256 b) public {
        bool r = a >= b;
	}

    function lessEq(uint256 a, uint256 b) public {
        bool r = a <= b;
	}
}
