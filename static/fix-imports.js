const fs = require('fs');
const path = require('path');

const distDir = './dist';

function fixImports(dir) {
  const files = fs.readdirSync(dir, { withFileTypes: true });
  
  for (const file of files) {
    const fullPath = path.join(dir, file.name);
    
    if (file.isDirectory()) {
      fixImports(fullPath);
    } else if (file.name.endsWith('.js')) {
      let content = fs.readFileSync(fullPath, 'utf8');
      
      // Fix import statements to include .js extension
      content = content.replace(
        /from ['"](\.\.?\/[^'"]+)['"]/g,
        (match, importPath) => {
          // Don't add .js if it already has an extension or is a package
          if (importPath.match(/\.(js|ts|json)$/)) {
            return match;
          }
          return `from '${importPath}.js'`;
        }
      );
      
      fs.writeFileSync(fullPath, content, 'utf8');
      console.log(`Fixed imports in: ${fullPath}`);
    }
  }
}

console.log('Fixing ES module imports...');
fixImports(distDir);
console.log('Done!');