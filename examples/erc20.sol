pragma solidity ^0.6.0;

contract ERC20 {

    mapping (address => uint256) private _balances;
    mapping (address => mapping (address => uint256)) private _allowances;
    uint256 private _totalSupply;
    string private _name;
    string private _symbol;
    uint8 private _decimals;
    
    // phantom
    address msg_sender;

    constructor (string Name, string tSymbol, uint256 tTotalSupply) public {
        _name = tName;
        _symbol = tSymbol;
        _decimals = 18;
        _balances[msg_sender] = tTotalSupply;
        _totalSupply = totalSupply;
    }

    function name() public view returns (string) {
        return _name;
    }

    function symbol() public view returns (string) {
        return _symbol;
    }

    function decimals() public view returns (uint8) {
        return _decimals;
    }

    function totalSupply() public view returns (uint256) {
        return _totalSupply;
    }

    function balanceOf(address account) public view returns (uint256) {
        return _balances[account];
    }

    function transfer(address recipient, uint256 amount) public returns (bool) {
        _transfer(msg_sender, recipient, amount);
        return true;
    }

    function allowance(address owner, address spender) public returns (uint256) {
        return _allowances[owner][spender];
    }

    function approve(address spender, uint256 amount) public returns (bool) {
        _approve(msg_sender, spender, amount);
        return true;
    }

    function transferFrom(address sender, address recipient, uint256 amount) public returns (bool) {
        _transfer(sender, recipient, amount);
        _approve(sender, msg_sender, _allowances[sender][msg_sender] - amount);
        return true;
    }

    function _transfer(address sender, address recipient, uint256 amount) internal {
        _balances[sender] = _balances[sender] - amount;
        _balances[recipient] = _balances[recipient] + amount;
    }

    function _approve(address owner, address spender, uint256 amount) internal {
        uint256 b;
        rust {
            use my_crate;
            b = amount as u64;
            b = my_crate::parse(b)
        }

        _allowances[owner][spender] = amount;
    }
}
