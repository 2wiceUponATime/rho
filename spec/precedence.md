Operators (highest to lowest precedence):

<table>
  <thead>
    <tr>
      <th>Precedence</th>
      <th>Associativity</th>
      <th>Operator</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td rowspan="7">13</td>
      <td rowspan="5">Left</td>
      <td>Member access <br/> <code>x.y</code></td>
    </tr>
    <tr>
      <td>Optional chaining <br/> <code>x?.y</code></td>
    </tr>
    <tr>
      <td>Index <br/> <code>x[y]</code></td>
    </tr>
    <tr>
      <td>Optional chaining index <br/> <code>x?.[y]</code></td>
    </tr>
    <tr>
      <td>Non-null assertion <br/> <code>x!</code></td>
    </tr>
    <tr>
      <td rowspan="2">N/A</td>
      <td>Function call <br/> <code>x(y)</code></td>
    </tr>
    <tr>
      <td>Optional chaining function call <br/> <code>x?.(y)</code></td>
    </tr>
    <tr>
      <td rowspan="4">12</td>
      <td rowspan="4">N/A</td>
      <td>Logical NOT <br/> <code>!x</code></td>
    </tr>
    <tr>
      <td>Bitwise NOT <br/> <code>~x</code></td>
    </tr>
    <tr>
      <td>Unary negation <br/> <code>-x</code></td>
    </tr>
    <tr>
      <td><code>await x</code></td>
    </tr>
    <tr>
      <td>11</td>
      <td>Right</td>
      <td>Exponentiation <br/> <code>x ** y</code></td>
    </tr>
    <tr>
      <td rowspan="3">10</td>
      <td rowspan="3">Left</td>
      <td>Multiplication <br/> <code>x * y</code></td>
    </tr>
    <tr>
      <td>Division <br/> <code>x / y</code></td>
    </tr>
    <tr>
      <td>Remainder <br/> <code>x % y</code></td>
    </tr>
    <tr>
      <td rowspan="2">9</td>
      <td rowspan="2">Left</td>
      <td>Addition <br/> <code>x + y</code></td>
    </tr>
    <tr>
      <td>Subtraction <br/> <code>x - y</code></td>
    </tr>
    <tr>
      <td rowspan="2">8</td>
      <td rowspan="2">Left</td>
      <td>Left shift <br/> <code>x << y</code></td>
    </tr>
    <tr>
      <td>Right shift <br/> <code>x >> y</code></td>
    </tr>
    <tr>
      <td>7</td>
      <td>Left</td>
      <td>Bitwise AND <br/> <code>x & y</code></td>
    </tr>
    <tr>
      <td>6</td>
      <td>Left</td>
      <td>Bitwise XOR <br/> <code>x ^ y</code></td>
    </tr>
    <tr>
      <td>5</td>
      <td>Left</td>
      <td>Bitwise OR <br/> <code>x | y</code></td>
    </tr>
    <tr>
      <td rowspan="5">4</td>
      <td rowspan="5">None - error on <code>x < y < z</code></td>
      <td>Less than <br/> <code>x < y</code></td>
    </tr>
    <tr>
      <td>Less than or equal <br/> <code>x <= y</code></td>
    </tr>
    <tr>
      <td>Greater than <br/> <code>x > y</code></td>
    </tr>
    <tr>
      <td>Greater than or equal <br/> <code>x >= y</code></td>
    </tr>
    <tr>
      <td><code>x is T</code></td>
    </tr>
    <tr>
      <td rowspan="2">3</td>
      <td rowspan="2">None - error on <code>x == y == z</code></td>
      <td>Equality <br/> <code>x == y</code></td>
    </tr>
    <tr>
      <td>Inequality <br/> <code>x != y</code></td>
    </tr>
    <tr>
      <td>2</td>
      <td>Left</td>
      <td>Logical AND <br/> <code>x && y</code></td>
    </tr>
    <tr>
      <td>1</td>
      <td>Left</td>
      <td>Logical OR <br/> <code>x || y</code></td>
    </tr>
    <tr>
      <td>0</td>
      <td>Left</td>
      <td>Nullish coalescing operator <br/> <code>x ?? y</code></td>
    </tr>
  </tbody>
</table>
