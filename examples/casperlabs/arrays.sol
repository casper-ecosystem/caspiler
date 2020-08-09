contract Contract {

    uint64[2][2] globalStaticNumbers;

	function staticOneDimLocalArray() public {
        uint64[5] numbers = [ uint64(2), 3, 5, 7, 11 ];
        numbers[1] = uint64(13);
	}

	function staticTwoDimsLocalArray() public {
        uint64[3][2] numbers = [ [ uint64(1), 2, 3], [6, 7, 8] ];
        numbers[0][1] = uint64(10);
	}

	function staticThreeDimsLocalArray() public {
        uint64[1][2][3] numbers = [ 
            [ [uint64(1)], [2] ], 
            [ [3], [4] ], 
            [ [5], [6] ]
        ];
	}

	function staticOneDimGlobalArray() public {
        globalStaticNumbers[1][1] = uint64(13);
	}
}
