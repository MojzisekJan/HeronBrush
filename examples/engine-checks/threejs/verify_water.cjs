// Runs the actual generated controller and shader using only vendored Three.js.
const {chromium}=require('playwright'),fs=require('node:fs'),http=require('node:http'),path=require('node:path'),assert=require('node:assert/strict');
(async()=>{
 const root=path.resolve(__dirname,'../../..'),html=fs.readFileSync(process.argv[2],'utf8').replaceAll('https://cdn.jsdelivr.net/npm/three@0.186.0/build/','/vendor/build/').replaceAll('https://cdn.jsdelivr.net/npm/three@0.186.0/examples/jsm/','/vendor/examples/jsm/');
 const server=http.createServer((req,res)=>{const url=new URL(req.url,'http://localhost');if(url.pathname.startsWith('/vendor/')){const file=path.resolve(root,'crates/heron-brush-web/vendor/three',url.pathname.slice('/vendor/'.length));if(!file.startsWith(path.join(root,'crates/heron-brush-web/vendor')+path.sep)||!fs.existsSync(file)){res.statusCode=404;return res.end();}res.setHeader('Content-Type','text/javascript');return res.end(fs.readFileSync(file));}res.setHeader('Content-Type','text/html');res.end(html);});
 await new Promise(r=>server.listen(0,'127.0.0.1',r));let browser;
 try{browser=await chromium.launch({headless:true,args:['--no-sandbox','--use-angle=swiftshader','--enable-unsafe-swiftshader']});const page=await browser.newPage({viewport:{width:960,height:640}}),errors=[];page.on('pageerror',e=>errors.push(e.message));page.on('console',m=>{if(m.type()==='error')errors.push(m.text());});await page.goto(`http://127.0.0.1:${server.address().port}/?verify=1`);await page.waitForFunction(()=>window.heronPlaytest,{timeout:30000});
 const code=fs.readFileSync(path.join(__dirname,'water_checks.js'),'utf8').replace('export function','function');
 const result=await page.evaluate(code+';runWaterCheck(window.heronPlaytest)');

 await page.waitForTimeout(150);await page.screenshot({path:'/tmp/heron-water-three.png'});assert.deepEqual(errors,[]);console.log('WATER_CHECK Three.js PASS',result);
 }finally{if(browser)await browser.close();server.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
